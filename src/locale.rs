use std::{
    collections::HashMap, sync::{Arc, LazyLock, Mutex}
};
use unic_langid::LanguageIdentifier;
use fluent_bundle::{FluentBundle, FluentResource, FluentArgs};

use crate::config;


include!(concat!(env!("OUT_DIR"), "/locale_data.rs")); // defines get_locale_resources and LANGUAGE_LIST

static LANGUAGE_LIST: LazyLock<Mutex<Vec<(String,String)>>> = LazyLock::new(||Mutex::new(init_language_list()));


fn init_language_list() -> Vec<(String,String)> {
    let mut languages = LOCALES.to_vec();

    // Move the default language to the top of the language list
    let default_language = &sys_locale::get_locale().unwrap_or_else(|| "en-GB".to_string());
    if let Some(pos) = languages.iter().position(|lang| lang == &default_language) {
        let default_lang = languages.remove(pos);
        languages.insert(0, default_lang);
    }

    let mut m = Vec::new();
    for lang in languages {
        m.push((lang.to_owned(),get_message(&get_locale(Some(lang)), "language-name", None)));
    }

    return m
}

pub fn get_message(locale: &FluentBundle<Arc<FluentResource>>, id: &str, args: Option<& FluentArgs<'_>>) -> String {
    if let Some(message) = locale.get_message(id) {
        if let Some(value) = message.value() {
            let mut err = vec![];
            return locale.format_pattern(value, args, &mut err).to_string();
        } else {
            return id.to_owned() // Return id if it is not available
        }
    } else {
        return id.to_owned(); // Return id if it is not available
    }
}

pub fn get_locale(lang: Option<&str>) -> FluentBundle<Arc<FluentResource>> {
    let locale = if let Some(locale) = lang {
        locale
    } else {
        // If language is not provided, get language from config
        if let Some(language) = config::get_config_string("language") {
            &language.clone()
        } else {
            // The language is not in the config file.
            &sys_locale::get_locale().unwrap_or_else(|| "en-GB".to_string()) // If locale cannot be identified, default to English
        }
        
        
    };
    
    let resource_data = if let Some(resources) = get_locale_resources(&locale) {
        resources
    } else {
        get_locale_resources("en-GB").unwrap() // Use English if the locale is not supported
    };

    let resource = FluentResource::try_new(resource_data).expect("Failed to parse FTL string.");
    
    let lang_id: LanguageIdentifier = locale.parse().unwrap_or_else(|_| "en-GB".parse().unwrap());
    let mut bundle = FluentBundle::new(vec![lang_id]);

    bundle.add_resource_overriding(resource.into());
    bundle
}

pub fn get_language_list() -> Vec<(String,String)> {
    LANGUAGE_LIST.lock().unwrap().clone()
}