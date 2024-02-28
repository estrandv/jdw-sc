
/*
    Sorts samples in pack by category using very barebones keyword detection on filenames.
 */
use std::collections::HashMap;

use crate::samples::Sample;

pub struct SampleCategoryDict {
    pub sample_map: HashMap<String, Vec<Sample>>
}

impl SampleCategoryDict {
    pub fn add(&mut self, sample: Sample) {

        let key = get_sample_category(&sample.file_name);

        let needs_vec = !self.sample_map.contains_key(&key);

        if needs_vec {
            self.sample_map.insert(key.to_string(), Vec::new());
        }

        self.sample_map.get_mut(&key).unwrap().push(sample);

    }
}

// For categorizing based on name, e.g. "hihat_88" -> category:"hh"
pub struct SampleCategory<'a> {
    pub key: &'a str,
    pub includes: Vec<&'a str>,
    pub excludes: Vec<&'a str>,
}

impl SampleCategory<'_> {
    pub fn accepts(&self, sample_name: &str) -> bool {
        self.includes.iter().any(|incl| sample_name.to_lowercase().contains(&incl.to_lowercase()))
            && !self.excludes.iter().any(|excl| sample_name.to_lowercase().contains(&excl.to_lowercase()))
    }
}

// Assign to a predetermined "category" that we can then use to call samples by type
pub fn get_sample_category(filename: &str) -> String {

    // TODO: Static
    let categories = vec![
        SampleCategory {
            key: "hh", includes: vec!["hat", "stick", "hh"],
            excludes: vec![]
        },
        SampleCategory {
            key: "bd", includes: vec!["bass", "drum", "kick", "bd"],
            excludes: vec!["crash"]
        },
        SampleCategory {
            key: "sh", includes: vec!["maraca", "shake", "tamb", "casta"],
            excludes: vec![]
        },
        SampleCategory {
            key: "to", includes: vec!["tom", "conga", "block", "bongo"],
            excludes: vec![]
        },
        SampleCategory {
            key: "sn", includes: vec!["snare", "clap", "sn", "sd"],
            excludes: vec![]
        },
        SampleCategory {
            key: "cy", includes: vec!["cymbal", "crash", "ride"],
            excludes: vec![]
        },
        SampleCategory {
            key: "be", includes: vec!["bell", "ring", "glass"],
            excludes: vec![]
        },
    ];

    let found = categories.iter().find(|cat| cat.accepts(filename));

    match found {
        Some(cat) => cat.key.to_string(),
        None => "mi".to_string() // "misc"
    }

}