/*
   Created notes often get assigned an external_id from the caller, which
       is then used to look up the actual nodeId used in the created internal
       supercollider osc message. IdRegistry keeps track of these variables.
*/
use std::cell::RefCell;
use std::collections::HashMap;

use log::{debug, warn};
use regex::Regex;

use crate::config;

pub struct NodeIDRegistry {
    pub registry: RefCell<HashMap<String, i32>>,
    curr_id: RefCell<i32>,
    regex_cache: RefCell<HashMap<String, Regex>>,
}

impl NodeIDRegistry {
    pub fn new() -> NodeIDRegistry {
        NodeIDRegistry {
            registry: RefCell::new(HashMap::new()),
            curr_id: RefCell::new(config::Config::get().first_node_id),
            regex_cache: RefCell::new(HashMap::new()),
        }
    }

    // Assign and return a new unique node_id for the given external_id
    pub fn create_node_id(&self, external_id: &str) -> Result<i32, String> {
        let mut node_id = *self.curr_id.borrow();
        node_id += 1;
        self.curr_id.replace(node_id);

        let with_id_fill = external_id.replace("{nodeId}", &node_id.to_string());

        if self.registry.borrow().contains_key(&with_id_fill) {
            let all_ids: Vec<String> = self.registry.borrow().keys().cloned().collect();
            debug!("[jdw-sc] External id conflict: '{}' (with nodeId fill: '{}'). Currently registered ({} ids): {:?}", external_id, with_id_fill, all_ids.len(), all_ids);
            return Err(format!("External id already taken: {}", external_id));
        }

        self.registry.borrow_mut().insert(with_id_fill, node_id);

        Ok(node_id)
    }

    // Clear all node_ids matching regex
    fn get_regex(&self, pattern: &str) -> Result<Regex, regex::Error> {
        if let Some(cached) = self.regex_cache.borrow().get(pattern) {
            return Ok(cached.clone());
        }
        let re = Regex::new(pattern)?;
        self.regex_cache.borrow_mut().insert(pattern.to_string(), re.clone());
        Ok(re)
    }

    pub fn regex_clear_node_ids(&self, external_id_regex: &str) {
        match self.get_regex(external_id_regex) {
            Ok(regex) => {
                self.registry.borrow_mut().retain(|entry, _| !regex.is_match(entry));
            }
            Err(_) => {
                warn!("Invalid regex: {}", external_id_regex);
            }
        }
    }

    pub fn regex_search_node_ids(&self, external_id_regex: &str) -> Vec<i32> {
        match self.get_regex(external_id_regex) {
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
        self.registry.borrow_mut().remove(&external_id);
    }
}
