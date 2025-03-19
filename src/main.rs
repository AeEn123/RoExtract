#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod log;
mod gui;
mod logic;
mod updater;
mod config;
mod locale;

use clap::{Parser, ValueEnum};

// CLI stuff
#[derive(ValueEnum, Clone, Debug)]
enum Category {
    Music,
    Sounds,
    Images,
    Ktx,
    Rbxm,
}

// Implement `Display` for `Category`

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// List assets
    #[arg(short, long)]
    list: bool,

    /// Set mode, using this is generally recommended, if this is not provided, the program will run the same function across each mode
    #[arg(short, long, value_name = "CATAGORY")]
    mode: Option<Category>,

    /// Extract asset, extract directory if no asset provided
    #[arg(short, long)]
    extract: Option<Option<String>>,

    /// Add a file extention automatically
    #[arg(long)]
    extention: bool,

    /// Define a destination path
    #[arg(short, long)]
    dest: Option<String>,

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

fn get_tab(category: Category) -> String {
    category.to_string().to_lowercase().replace("ktx","ktx-files").replace("rbxm","rbxm-files")
}

fn list(tab: String) {
    let cache_directory = {
        let cache_dir = logic::get_cache_directory();
        // Music tab just adds .ogg while other tabs scrape the header files from HTTP to allow all media players to play it
        if tab == "music" {
            format!("{}/sounds", cache_dir)
        } else {
            format!("{}/http", cache_dir)
        }
    };
    logic::refresh(cache_directory, tab, true, true); // cli_list_mode is set to true, this will print assets to console
}

fn extract(tab: String, asset: Option<String>, destination: Option<String>, add_extention: bool) {
    let cache_directory = {
        let cache_dir = logic::get_cache_directory();
        // Music tab just adds .ogg while other tabs scrape the header files from HTTP to allow all media players to play it
        if tab == "music" {
            format!("{}/sounds", cache_dir)
        } else {
            format!("{}/http", cache_dir)
        }
    };
    if let Some(asset) = asset {
        let dest = destination.unwrap_or(asset.clone());
        logic::extract_file(format!("{}/{}", cache_directory, asset), tab, dest, add_extention);
    } else {
        if let Some(dest) = destination {
            logic::refresh(cache_directory.clone(), tab.clone(), true, true);
            logic::extract_dir(cache_directory, dest, tab, true, false);
        } else {
            eprintln!("Please provide either a destination path or an asset to extract! --help for more details.")
        }

    }

}

fn main() {
    let args = Cli::parse();

    if args.list {
        if let Some(category) = args.mode {
            list(get_tab(category));
        } else {
            // Not enough arguments - go through all
            for category in logic::get_categories() {
                list(category);
            }
        }


    } else if let Some(asset) = args.extract  {
        if let Some(category) = args.mode {
            extract(get_tab(category), asset, args.dest, args.extention);
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
            let dir = match args.mode {
                Some(Category::Music) => &format!("{}/sounds", &logic::get_cache_directory()), //sounds if music
                _ => &format!("{}/http", &logic::get_cache_directory())
            };

            logic::swap_assets(dir, &asset, &dest);
        } else {
            eprintln!("--dest is required for swapping assets, --help for more details")
        }
    } else if args.cache_dir {
        println!("{}", logic::get_cache_directory());
    } else if args.check_for_updates {
        updater::check_for_updates(false, false);
    } else if args.download_new_update {
        updater::check_for_updates(false, true);
    } else {
        // If nothing passed, run GUI
        gui::run_gui();
    }
    
    if !updater::run_install_script(false) {
        // Only run if the install script hasn't ran
        logic::clean_up(); // Remove the temporary directory if one has been created
    }
    
}
