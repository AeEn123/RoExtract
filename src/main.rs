#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod log;
mod gui;
mod logic;
mod updater;
mod config;
mod locale;

use std::path::PathBuf;

use clap::Parser;

use crate::logic::extract_to_file;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// List assets
    #[arg(short, long)]
    list: bool,

    /// Set mode, using this is generally recommended, if this is not provided, the program will run the same function across each mode
    #[arg(short, long, value_name = "CATEGORY")]
    mode: Option<logic::Category>,

    /// Extract asset, extract directory if no asset provided
    #[arg(short, long)]
    extract: Option<Option<String>>,

    /// Add a file extension automatically
    #[arg(long)]
    extension: bool,

    /// Define a destination path
    #[arg(short, long)]
    dest: Option<PathBuf>,

    /// Swap two assets
    #[arg(short, long)]
    swap: Option<String>,

    /// Return the cache directory
    #[arg(short, long)]
    cache_dir: bool,

    /// Connect to the internet to check for updates
    #[arg(long)]
    check_for_updates: bool,

    /// Connect to the internet to download new update binary
    #[arg(long)]
    download_new_update: bool,
    
}

fn list(category: logic::Category) {
    logic::refresh(category, true, true); // cli_list_mode is set to true, this will print assets to console
}

fn extract(category: logic::Category, asset: Option<String>, destination: Option<PathBuf>, add_extension: bool) {
    if let Some(asset) = asset {
        let dest = destination.unwrap_or(asset.clone().into());
        let info = logic::create_asset_info(&asset, category);
        match logic::extract_to_file(info, dest, add_extension) {
            Ok(destination) => println!("{}", destination.display()),
            Err(e) => eprintln!("{}", e)
        }

    } else {
        if let Some(dest) = destination {
            logic::refresh(category, true, true);
            logic::extract_dir(dest, category, true, false);
        } else {
            eprintln!("Please provide either a destination path or an asset to extract! --help for more details.")
        }

    }

}

fn main() {
    let args = Cli::parse();

    if args.list {
        if let Some(category) = args.mode {
            list(category);
        } else {
            list(logic::Category::All);
            list(logic::Category::Music)
        }
    } else if let Some(asset) = args.extract  {
        if let Some(category) = args.mode {
            extract(category, asset, args.dest, args.extension);
        } else if let Some(asset) = asset {
            // User passed a single asset without mode, determine category.
            let info = logic::create_asset_info(&asset, logic::Category::All);
            let category = logic::determine_category(
                &logic::extract_asset_to_bytes(info).unwrap_or(Vec::new())
            );

            let info = logic::create_asset_info(&asset, category);
            
            match extract_to_file(info, if let Some(destination) = args.dest { destination } else { asset.into() }, args.extension) {
                Ok(destination) => println!("{}", destination.display()),
                Err(e) => eprintln!("{}", e)
            }
        } else {
            // Not enough arguments - go through all
            if let Some(destination) = args.dest {
                logic::extract_all(destination, true, false);
            } else {
                eprintln!("--dest is required to extract all assets. --help for more details")
            }

        }
    } else if let Some(asset) = args.swap {
        if let Some(dest) = args.dest {
            let asset_a = logic::create_asset_info(&asset, args.mode.unwrap_or(logic::Category::All));
            let asset_b = logic::create_asset_info(&dest.to_string_lossy().to_string(), args.mode.unwrap_or(logic::Category::All));

            logic::swap_assets(asset_a, asset_b);
        } else {
            eprintln!("--dest is required for swapping assets, --help for more details")
        }
    } else if args.cache_dir {
        println!("{}", logic::cache_directory::get_cache_directory().display());
    } else if args.check_for_updates {
        updater::check_for_updates(false, false);
    } else if args.download_new_update {
        updater::check_for_updates(false, true);
    } else {
        // If nothing passed, run GUI
        gui::run_gui();
    }
    
    // The program is now closing
    config::save_config_file();
    
    if !updater::run_install_script(false) {
        // Only run if the install script hasn't ran
        logic::clean_up(); // Remove the temporary directory if one has been created
    }
    
}
