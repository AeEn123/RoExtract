//! Audio lengths, worked out in the background and cached by asset name.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::Cursor,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Condvar, LazyLock, Mutex,
    },
    thread,
    time::Duration,
};

use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

use crate::logic::{self, AssetInfo};

/// Most threads working out lengths at once.
const MAX_WORKERS: usize = 4;

struct Lengths {
    /// None when the length couldn't be worked out.
    known: HashMap<String, Option<Duration>>,
    queued: HashSet<String>,
    queue: VecDeque<AssetInfo>,
}

static LENGTHS: LazyLock<Mutex<Lengths>> = LazyLock::new(|| {
    Mutex::new(Lengths {
        known: HashMap::new(),
        queued: HashSet::new(),
        queue: VecDeque::new(),
    })
});
static WORK_AVAILABLE: Condvar = Condvar::new();
static WORKERS: AtomicUsize = AtomicUsize::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Length of the audio in `bytes`, None if it isn't audio or can't be read.
pub fn length_of(bytes: Vec<u8>, format: Option<&str>) -> Option<Duration> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if let Some(format) = format {
        hint.with_extension(format);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let mut reader = probed.format;
    let track = reader.default_track()?;
    let params = &track.codec_params;

    if let (Some(frames), Some(rate)) = (params.n_frames, params.sample_rate) {
        return Some(Duration::from_secs_f64(frames as f64 / rate as f64));
    }

    // Not in the headers: add up the packets, without decoding them
    let time_base = params.time_base?;
    let track_id = track.id;
    let mut total = 0;
    while let Ok(packet) = reader.next_packet() {
        if packet.track_id() == track_id {
            total += packet.dur;
        }
    }
    let time = time_base.calc_time(total);
    Some(Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac))
}

/// The length of `asset` if known. Otherwise queues it to be worked out and
/// returns None; `generation` changes once it's done.
pub fn length(asset: &AssetInfo) -> Option<Duration> {
    let mut lengths = LENGTHS.lock().unwrap();
    if let Some(length) = lengths.known.get(&asset.name) {
        return *length;
    }
    if lengths.queued.insert(asset.name.clone()) {
        lengths.queue.push_back(asset.clone());
        drop(lengths);
        start_worker();
        WORK_AVAILABLE.notify_one();
    }
    None
}

/// The length of `asset` if already known, without queueing it.
pub fn cached_length(asset: &AssetInfo) -> Option<Duration> {
    LENGTHS
        .lock()
        .unwrap()
        .known
        .get(&asset.name)
        .copied()
        .flatten()
}

/// True while `asset`'s length is being worked out.
pub fn is_pending(asset: &AssetInfo) -> bool {
    LENGTHS.lock().unwrap().queued.contains(&asset.name)
}

/// Changes whenever a length is worked out.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

/// Forget all lengths, e.g. after the cache was cleared or assets were swapped.
pub fn clear() {
    let mut lengths = LENGTHS.lock().unwrap();
    lengths.known.clear();
    lengths.queue.clear();
    lengths.queued.clear();
    GENERATION.fetch_add(1, Ordering::Relaxed);
}

fn start_worker() {
    let started = WORKERS.fetch_update(Ordering::AcqRel, Ordering::Acquire, |workers| {
        (workers < MAX_WORKERS).then_some(workers + 1)
    });
    if started.is_ok() {
        thread::spawn(worker);
    }
}

fn worker() {
    loop {
        let asset = {
            let mut lengths = LENGTHS.lock().unwrap();
            loop {
                if let Some(asset) = lengths.queue.pop_front() {
                    break asset;
                }
                // Idle workers exit after a while; new ones start when needed.
                let (guard, timeout) = WORK_AVAILABLE
                    .wait_timeout(lengths, Duration::from_secs(5))
                    .unwrap();
                lengths = guard;
                if timeout.timed_out() && lengths.queue.is_empty() {
                    WORKERS.fetch_sub(1, Ordering::AcqRel);
                    return;
                }
            }
        };

        let length = logic::extract_asset_to_bytes(asset.clone())
            .ok()
            .and_then(|bytes| length_of(bytes, asset.format));

        let mut lengths = LENGTHS.lock().unwrap();
        // Skip if cleared meanwhile
        if lengths.queued.remove(&asset.name) {
            lengths.known.insert(asset.name, length);
            GENERATION.fetch_add(1, Ordering::Relaxed);
            drop(lengths);
            logic::request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_of_non_audio_is_none() {
        assert_eq!(length_of(b"not audio at all".to_vec(), Some("ogg")), None);
        assert_eq!(length_of(Vec::new(), None), None);
    }
}
