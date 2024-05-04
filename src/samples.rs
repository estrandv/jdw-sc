use std::collections::HashMap;
use std::fmt::format;
use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use rosc::OscType;

use crate::config::{SERVER_NAME, SERVER_OSC_SOCKET_NAME};
use crate::PlaySampleMessage;
use crate::sample_sorting::SampleCategoryDict;
use crate::util::Counter;

#[derive(Debug, Clone)]
pub struct Sample {
    pub file_name: String, // e.g. "hihat88.wav"
    pub buffer_nr: i32,
}

impl Sample {

    // Typical "read into buffer" script to be run on server boot
    pub fn to_buffer_load_scd(&self, dir: &str) -> String {
        format!(
            "Buffer.read({}, \"{}\", 0, -1, bufnum: {}); \n",
            SERVER_NAME,
            dir.to_string() + "/" + &self.file_name.to_string(),
            self.buffer_nr
        )
    }

}

pub struct SamplePack {
    pub dir_path: PathBuf, // e.g. "wav/example"
    pub samples: Vec<Sample>,
    pub sample_dict: HashMap<String, Vec<Sample>> // samples by category
}

impl SamplePack {

    pub fn get_dir_path(&self) -> &str {
        self.dir_path.file_name().unwrap().to_str().unwrap()
    }

    // TODO: Some scoping confusion here - impl functions like these in the appropriate places! 
    pub fn as_buffer_load_scd(&self) -> String {
        let mut script = "".to_string();
        let dir = self.dir_path.to_str().unwrap();
        for sample in &self.samples {
            script += &sample.to_buffer_load_scd(dir)
        }
        debug!("Buffer load scd string for sample pack: {}", &script);
        script.to_string()
    }

    pub fn as_nrt_buffer_load_rows(&self) -> Vec<String> {
        let dir = self.dir_path.to_str().unwrap();

        self.samples.iter()
            .map(|s|s.to_nrt_scd_row(dir))
            .collect()
    }

    pub fn category_to_buf(&self, number: usize, category: Option<String>) -> i32 {

        if category.is_some() {

            let cat = category.clone().unwrap().to_string();
            let sub_pack = self.sample_dict.get(&cat);

            if sub_pack.is_some() {

                let pack_max_index = sub_pack.unwrap().len();

                let index = number % pack_max_index;

                let samples = sub_pack.unwrap().clone();

                info!("Resolved buffer number for cat {}", &cat);

                return samples.get(index).unwrap().buffer_nr;
            }
            else {
                info!("Cannot find requested category {}, defaulting to pack index for sample play", category.clone().unwrap());
            }

        }

        info!("Request did not provide any category key, playing sample as buffer index");

        let index = number % self.samples.len();
        return self.samples.get(index).unwrap().buffer_nr;
    }
}

pub struct SamplePackCollection {
    pub sample_packs: HashMap<String, SamplePack>,
    pub counter: Counter
}

impl SamplePackCollection {

    pub fn create(dir: &Path) -> Result<SamplePackCollection, String> {

        let mut counter = Counter {value: -1};

        let mut packs: HashMap<String, SamplePack> = HashMap::new();

        if !dir.exists() {
            warn!("Samples dir {:?} does not exist, skipping sample loading...", &dir);
            return Ok(SamplePackCollection {
                sample_packs: packs,
                counter
            });
        }

        for entry in fs::read_dir(dir).unwrap() {
            let path = match entry {
                Ok(e) => {Ok(e)}
                Err(e) => {Err(format!("File read error: {}", e))}
            }?.path();
            if path.is_dir() {

                // Each found subfolder is treated as a sample pack
                let mut samples: Vec<Sample> = Vec::new();
                let mut sample_sorter = SampleCategoryDict {sample_map: HashMap::new()};

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

                    if name.contains(".wav") || name.contains(".WAV") {
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

                info!("Creating sample pack with name {} and path {:?}", &pack_name, &path);
                packs.insert(pack_name, SamplePack{
                    dir_path: path,
                    samples,
                    sample_dict: sample_sorter.sample_map
                });

            }
        }

        Ok(SamplePackCollection {
            sample_packs: packs,
            counter
        })

    }

    pub fn empty() -> SamplePackCollection {
        SamplePackCollection {
            sample_packs: HashMap::new(),
            counter: Counter{value: 1}
        }
    }

    pub fn category_to_buf(&self, pack: &str, number: usize, category: Option<String>) -> Option<i32> {
        let pack= self.sample_packs.get(pack);

        match pack {
            Some(sp) => {
                Some(sp.category_to_buf(number, category))
            },
            None => {
                return None;
            }
        }

    }

    pub fn as_buffer_load_scd(&self) -> String {
        let vec = self.sample_packs.values().clone().collect::<Vec<&SamplePack>>();
        let vector = vec.iter().map(|pack| pack.as_buffer_load_scd()).collect::<Vec<String>>();
        let result = vector.join("\n") + "\n" + SERVER_OSC_SOCKET_NAME + ".sendMsg(\"/buffers_loaded\", \"ok\");";
        result
    }

    // NOTE: Belonging of all conversion methods is a bit unclear.
    // For now, they work here, but ideally samples should not care for neither nrt nor buffer scd strings
    pub fn as_nrt_buffer_load_rows(&self) -> Vec<String> {
        self.sample_packs.values()
            .flat_map(|pack| pack.as_nrt_buffer_load_rows())
            .collect()
    }


}
