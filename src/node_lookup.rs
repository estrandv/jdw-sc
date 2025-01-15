/*
   Created notes often get assigned an external_id from the caller, which
       is then used to look up the actual nodeId used in the created internal
       supercollider osc message. IdRegistry keeps track of these variables.
*/
use std::cell::RefCell;
use std::collections::HashMap;

use log::{debug, warn};
use regex::Regex;

pub struct NodeIDRegistry {
    pub registry: RefCell<HashMap<String, i32>>,
    curr_id: RefCell<i32>,
}

impl NodeIDRegistry {
    pub fn new() -> NodeIDRegistry {
        NodeIDRegistry {
            registry: RefCell::new(HashMap::new()),
            curr_id: RefCell::new(100),
        }
    }

    // Assign and return a new unique node_id for the given external_id
    pub fn create_node_id(&self, external_id: &str) -> i32 {
        let mut node_id = self.curr_id.clone().into_inner();
        node_id += 1;

        let mut new_reg = self.registry.clone().into_inner();
        new_reg.insert(external_id.to_string(), node_id);
        self.registry.replace(new_reg);

        self.curr_id.replace(node_id);

        node_id
    }

    // Clear all node_ids matching regex
    pub fn regex_clear_node_ids(&self, external_id_regex: &str) {
        let regex_attempt = Regex::new(external_id_regex);

        match regex_attempt {
            Ok(regex) => {
                let mut new_reg = self.registry.clone().into_inner();
                new_reg.retain(|entry, _| !regex.is_match(&entry));
                self.registry.replace(new_reg);
            }
            Err(_) => {
                warn!("Provided regex {} is invalid", external_id_regex);
            }
        }
    }

    // Get all node_ids matching regex
    pub fn regex_search_node_ids(&self, external_id_regex: &str) -> Vec<i32> {
        let regex_attempt = Regex::new(external_id_regex);

        match regex_attempt {
            Ok(regex) => {
                let matching: Vec<_> = self
                    .registry
                    .borrow()
                    .iter()
                    .filter(|entry| regex.is_match(entry.0))
                    .map(|entry| entry.1.clone())
                    .collect();

                debug!(
                    "Found {} running notes matching regex {}",
                    matching.len(),
                    external_id_regex
                );

                return matching;
            }
            Err(_) => {
                warn!("Provided regex {} is invalid", external_id_regex);
                vec![]
            }
        }
    }

    // Remove an external_id's node_id from the registry, if present
    #[allow(dead_code)]
    pub fn clear(&self, external_id: String) {
        if self.registry.borrow().contains_key(&external_id) {
            let mut new_reg = self.registry.clone().into_inner();
            new_reg.remove(&external_id);
            self.registry.replace(new_reg);
        }
    }
}
