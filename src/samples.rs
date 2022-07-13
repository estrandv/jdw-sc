use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use std::fmt::format;
use std::io::Error;
use std::sync::{Arc, Mutex};
use log::{debug, info};
use rosc::OscType;
use crate::config::{SERVER_NAME, SERVER_OSC_SOCKET_NAME};
use crate::PlaySampleMessage;

#[derive(Debug, Clone)]
pub struct Sample {
    pub file_name: String, // e.g. "hihat88.wav"
    pub buffer_nr: i32,
}

impl Sample {

    // Typical "read into buffer" script to be run on server boot
    pub fn to_buffer_load_scd(&self, dir: &str) -> String {
        format!(
            "Buffer.read({}, File.getcwd +/+ \"{}\", 0, -1, bufnum: {}); \n",
            SERVER_NAME,
            dir.to_string() + "/" + &self.file_name.to_string(),
            self.buffer_nr
        )
    }

    // Buffer load as-osc, suitable for loading into the NRT server
    pub fn to_nrt_scd_row(&self, dir: &str) -> String {
        // TODO: TEmplate-friendly pieces
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

    pub fn get_file_name(&self) -> &str {
        self.dir_path.file_name().unwrap().to_str().unwrap()
    }

    pub fn to_buffer_load_scd(&self) -> String {
        let mut script = "".to_string();
        let dir = self.dir_path.to_str().unwrap();
        for sample in &self.samples {
            script += &sample.to_buffer_load_scd(dir)
        }
        script.to_string()
    }

    pub fn get_buffer_number(&self, number: usize, category: Option<String>) -> i32 {

        if category.is_some() {

            let cat = category.clone().unwrap().to_string();
            let sub_pack = self.samples_ordered.get(&cat);

            if sub_pack.is_some() {

                let pack_max_index = sub_pack.unwrap().len();

                let index = ( (number) % (pack_max_index) );

                let samples = sub_pack.unwrap().clone();
                return samples.get(index).unwrap().buffer_nr;
            }
            else {
                info!("Cannot find requested category {}, defaulting to pack index for sample play", category.clone().unwrap());
            }

        }

        let index = (number % (self.samples.len()));
        return self.samples.get(index).unwrap().buffer_nr;
    }
}

// TODO: Can be in generic util.rs
pub struct Counter {
    value: i32
}

impl Counter {

    pub fn next(&mut self) -> i32{
        self.value += 1;
        self.value
    }
}

/*
    Sorts samples in pack by category using very barebones keyword detection on filenames.
 */
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
    pub sample_packs: HashMap<String, SamplePack>,
    pub counter: Counter
}


impl SampleDict {

    pub fn dummy() -> SampleDict {
        SampleDict {
            sample_packs: HashMap::new(),
            counter: Counter{value: 1}
        }
    }

    pub fn get_buffer_number(&self, pack: &str, number: usize, category: Option<String>) -> Option<i32> {
        let pack= self.sample_packs.get(pack);

        match pack {
            Some(sp) => {
                Option::Some(sp.get_buffer_number(number, category))
            },
            None => Option::None
        }

    }

    pub fn to_buffer_load_scd(&self) -> String {
        let vec = self.sample_packs.values().clone().collect::<Vec<&SamplePack>>();
        let vector = vec.iter().map(|pack| pack.to_buffer_load_scd()).collect::<Vec<String>>();
        let result = vector.join("\n") + "\n" + SERVER_OSC_SOCKET_NAME + ".sendMsg(\"/buffers_loaded\", \"ok\");";
        result
    }

    pub fn from_dir(dir: &Path) -> Result<SampleDict, String> {

        let mut counter = Counter {value: -1};

        let mut packs: HashMap<String, SamplePack> = HashMap::new();

        for entry in fs::read_dir(dir).unwrap() {
            let path = match entry {
                Ok(e) => {Ok(e)}
                Err(e) => {Err(format!("File read error: {}", e))}
            }?.path();
            if path.is_dir() {

                // Each found subfolder is treated as a sample pack
                let mut samples: Vec<Sample> = Vec::new();
                let mut sample_sorter = SampleSorter {sample_map: HashMap::new()};

                let read_subdir = match fs::read_dir(path.clone()) {
                    Ok(d) => {Ok(d)}
                    Err(e) => {Err(format!("IO Error {}", e))}
                }?;

                let files_in_dir_scan: Vec<Result<String, String>> = read_subdir
                    .map(|e| {

                        let res = match e {
                            Ok(r) => {r
                                .file_name()
                                .to_str()
                                .map(|s| s.to_string())
                                .ok_or("File name unreadable".to_string())}
                            Err(err) => {Err(format!("IO error {}", err))}
                        };

                        return res;

                    }).collect();

                let mut files_in_dir: Vec<String> = Vec::new();
                for result in files_in_dir_scan {
                    files_in_dir.push(result?.to_string());
                }

                files_in_dir.sort(); // Order by name

                // Each file in a subfolder is treated as a sample
                for name in files_in_dir {

                    if name.contains(".wav") {
                        let buffer_nr = counter.next();

                        let sample = Sample {
                            file_name: name.clone(),
                            buffer_nr
                        };

                        debug!("Adding sample {} as buf number {}", name, buffer_nr);

                        samples.push(sample.clone());

                        sample_sorter.add(sample);

                    } else {
                        debug!("Ignoring sample file {}; invalid format", name.clone());
                    }

                }

                let pack_name = path.file_name().ok_or("dir is nameless")?
                    .to_str().ok_or("dir name unreadable")?
                    .to_string();

                packs.insert(pack_name, SamplePack{
                    dir_path: path,
                    samples,
                    samples_ordered: sample_sorter.sample_map
                });

            }
        }

        Ok(SampleDict {
            sample_packs: packs,
            counter
        })
        
    }


}

impl PlaySampleMessage {
    pub fn get_args_with_buf(&self, samples: Arc<Mutex<SampleDict>>) -> Vec<OscType> {
        let mut base_args = self.args.clone();

        let buf_nr = samples
            .lock()
            .unwrap()
            .get_buffer_number(&self.sample_pack, self.index, self.category.clone())
            .unwrap_or(0); // Should probably be some kind of error, but for now default to base buf

        // TODO: Buf might already be in it. Might be good to wipe it.
        base_args.push(OscType::String("buf".to_string()));
        base_args.push(OscType::Int(buf_nr));

        base_args
    }
}
