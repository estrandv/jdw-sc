use std::collections::HashMap;

use crate::config::SERVER_NAME;
use crate::osc_model::LoadSampleMessage;

#[derive(Debug, Clone)]
pub struct Sample {
    pub file_path: String,
    pub buffer_number: i32,
    pub category_tag: String,
    pub tone_index: i32,
}

#[derive(Clone)]
pub struct SamplePack {
    pub samples: Vec<Sample>
}

#[derive(Clone)]
pub struct SamplePackDict {
    sample_packs: HashMap<String, SamplePack>
}

impl Sample {
    pub fn get_buffer_load_scd(&self) -> String {
        format!(
            "Buffer.read({}, \"{}\", 0, -1, bufnum: {}); \n",
            SERVER_NAME,
            self.file_path.to_string(),
            self.buffer_number
        )
    }

    pub fn get_nrt_scd_row(&self) -> String {
        let ret = format!(
            "[0.0, (Buffer.new(server, 44100 * 8.0, 2, bufnum: {})).allocReadMsg(\"{}\")]",
            self.buffer_number,
            self.file_path.to_string(),
        );

        ret
    }

}

impl SamplePack {

    pub fn find(
        &self,
        sample_number: usize,
        category: &str
    ) -> Option<Sample> {

        let samples_in_category: Vec<Sample> = self.samples.iter()
            // Return ALL samples if category is blank
            .filter(|sample| sample.category_tag == category || category == "")
            .map(|f| f.clone())
            .collect();

        return samples_in_category.iter()
            .find(|sample| sample.tone_index as usize == sample_number)
            .map(|sample| sample.clone());

    }
}

impl SamplePackDict {
    pub fn new() -> SamplePackDict {
        SamplePackDict {
            sample_packs: HashMap::new(),
        }
    }

    pub fn register_sample(&mut self, msg: LoadSampleMessage) -> Result<Sample, String> {


        let present = self.sample_packs.contains_key(&msg.sample_pack);

        if !present {
            self.sample_packs.insert(msg.sample_pack.to_string(), SamplePack{
                samples: Vec::new()
            });
        }

        let pack = self.sample_packs.get_mut(&msg.sample_pack).unwrap();

        // Remove existing samples with the same tone index (overwrite)
        pack.samples.retain(|sample| sample.tone_index != msg.tone_index);

        let existing = pack.samples.iter().find(|s| s.file_path == msg.file_path).map(|s| s.clone());


        // TODO: Might be a good idea to fail if present, or rewrite the buffer number completely
        let sample = Sample {
            file_path: msg.file_path.to_string(),
            buffer_number: msg.buffer_number,
            category_tag: msg.category_tag.to_string(),
            tone_index: msg.tone_index,
        };

        if !existing.is_some() {
            pack.samples.push(sample.clone());
        }

        let selected = if existing.is_some() {existing.unwrap()} else {sample};

        // TODO: Some more error handling is probably a good idea, like for duplicate buffer numbers
        // TODO: ref of sample might be enough of a return
        Ok(selected)
    }

    pub fn find(
        &self,
        sample_pack: &str,
        sample_number: usize,
        category: &str
    ) -> Option<Sample> {

        self.sample_packs.get(sample_pack).and_then(|pack| pack.find(sample_number, category))
    }

    pub fn get_all_samples(&self) -> Vec<Sample> {
        return self.sample_packs.values().flat_map(|pack| pack.samples.clone()).collect()
    }

}
