use std::collections::HashMap;
use crate::osc_model::LoadSampleMessage;

#[derive(Debug, Clone)]
pub struct Sample {
    pub file_path: String,
    pub buffer_number: i32,
    pub category_tag: String,
}

pub struct SamplePack {
    pub samples: Vec<Sample>
}

pub struct SamplePackDict {
    sample_packs: HashMap<String, SamplePack>
}

impl SamplePack {

    pub fn find(
        &self,
        sample_number: usize,
        category: &str
    ) -> Sample {
        let pack_max_index = self.samples.len();
        let index = sample_number % pack_max_index;
        return if !category.is_empty() {
            let samples_in_category: Vec<Sample> = self.samples.iter()
                .filter(|sample| sample.category_tag == category)
                .map(|sample| sample.clone())
                .collect();

            samples_in_category.get(index).unwrap().clone()
        } else {
            self.samples.get(index).unwrap().clone()
        }

    }
}

impl SamplePackDict {
    pub fn new() -> SamplePackDict {
        SamplePackDict {
            sample_packs: HashMap::new(),
        }
    }

    pub fn register_sample(&mut self, msg: LoadSampleMessage) -> Result<(), String> {

        let present = self.sample_packs.contains_key(&msg.sample_pack);

        if !present {
            self.sample_packs.insert(msg.sample_pack.to_string(), SamplePack{
                samples: Vec::new()
            });
        }

        let pack = self.sample_packs.get_mut(&msg.sample_pack).unwrap();

        pack.samples.push(Sample {
            file_path: msg.file_path.to_string(),
            buffer_number: msg.buffer_number,
            category_tag: msg.category_tag.to_string(),
        });

        // TODO: Some more error handling is probably a good idea, like for duplicate buffer numbers
        Ok(())
    }

    pub fn find(
        &self,
        sample_pack: &str,
        sample_number: usize,
        category: &str
    ) -> Sample {

        let found = self.sample_packs.get(sample_pack).unwrap().find(
            sample_number,
            category
        );

        found
    }

}