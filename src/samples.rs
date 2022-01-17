use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Sample {
    pub file_name: String, // e.g. "hihat88.wav"
    pub buffer_nr: i32,
}

impl Sample {

    // Typical "read into buffer" script to be run on server boot
    pub fn to_buffer_load_scd(&self, dir: &str) -> String {
        format!(
            "Buffer.read(s, File.getcwd +/+ \"{}\", 0, -1, bufnum: {}); \n",
            dir.to_string() + "/" + &self.file_name.to_string(),
            self.buffer_nr
        )
    }

    // Buffer load as-osc, suitable for loading into the NRT server
    pub fn to_nrt_scd_row(&self, dir: &str) -> String {
        format!(
            "[0.0, (Buffer.new(server, 44100 * 8.0, 2, bufnum: {})).allocReadMsg(File.getcwd +/+ \"{}\")]",
            dir.to_string() + &self.file_name.to_string(),
            self.buffer_nr
        )
    }
}

pub struct SamplePack {
    pub dir_path: PathBuf, // e.g. "wav/example"
    pub samples: Vec<Sample>,
    pub samples_ordered: HashMap<String, Vec<Sample>> // samples by category
}

impl SamplePack {
    pub fn to_buffer_load_scd(&self) -> String {
        let mut script = "".to_string();
        let dir = self.dir_path.to_str().unwrap();
        for sample in &self.samples {
            script += &sample.to_buffer_load_scd(dir)
        }
        script.to_string()
    }
}

pub struct Counter {
    value: i32
}

impl Counter {

    pub fn next(&mut self) -> i32{
        self.value += 1;
        self.value
    }
}

struct SampleSorter {
    pub sample_map: HashMap<String, Vec<Sample>>
}

impl SampleSorter {
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
struct SampleCategory<'a> {
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
fn get_sample_category(filename: &str) -> String {

    // TODO: Static
    let categories = vec![
        SampleCategory {
            key: "hh", includes: vec!["hat", "stick", "hh"],
            excludes: vec![]
        },
        SampleCategory {
            key: "bd", includes: vec!["bass", "drum", "kick"],
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
            key: "sn", includes: vec!["snare", "clap"],
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

pub struct SampleDict {
    pub sample_packs: Vec<SamplePack>,
    pub counter: Counter
}


impl SampleDict {

    pub fn to_buffer_load_scd(&self) -> String {
        let vector = self.sample_packs.iter().map(|pack| pack.to_buffer_load_scd()).collect::<Vec<String>>();
        vector.join("\n") + "\no.sendMsg(\"/buffers_loaded\", \"ok\");"
    }

    // TODO: Result return to avoid IO errors crashing everything
    pub fn from_dir(dir: &Path) -> SampleDict {

        let mut counter = Counter {value: -1};

        let mut packs: Vec<SamplePack> = Vec::new();

        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {

                // Each found subfolder is treated as a sample pack
                let mut samples: Vec<Sample> = Vec::new();
                let mut sample_sorter = SampleSorter {sample_map: HashMap::new()};

                // Each file in a subfolder is treated as a sample
                for sample_entry in fs::read_dir(path.clone()).unwrap() {
                    let sample_path = sample_entry.unwrap().path();
                    let name = sample_path.file_name().unwrap()
                        .to_str().unwrap()
                        .to_string();

                    let buffer_nr = counter.next();

                    if name.contains(".wav") {

                        let sample = Sample {
                            file_name: name,
                            buffer_nr
                        };

                        samples.push(sample.clone());

                        sample_sorter.add(sample);

                    }

                }

                packs.push(SamplePack{
                    dir_path: path,
                    samples,
                    samples_ordered: sample_sorter.sample_map
                })


            }
        }

        SampleDict {
            sample_packs: packs,
            counter
        }
        
    }
}