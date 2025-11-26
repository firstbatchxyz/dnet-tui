use std::collections::{HashMap, HashSet};

pub fn format_layers(layers: &[u32]) -> String {
    if layers.is_empty() {
        return "[]".to_string();
    }

    let mut sorted = layers.to_vec();
    sorted.sort_unstable();

    let mut ranges = Vec::new();
    let mut start = sorted[0];
    let mut end = sorted[0];

    for &layer in &sorted[1..] {
        if layer == end + 1 {
            end = layer;
        } else {
            if start == end {
                ranges.push(start.to_string());
            } else {
                ranges.push(format!("{}-{}", start, end));
            }
            start = layer;
            end = layer;
        }
    }

    if start == end {
        ranges.push(start.to_string());
    } else {
        ranges.push(format!("{}-{}", start, end));
    }

    ranges.join(",")
}

pub fn parse_layer_input(input: &str, max_layers: u32) -> Option<Vec<u32>> {
    let mut layers = Vec::new();

    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(dash_pos) = part.find('-') {
            // Range
            let start_str = &part[..dash_pos].trim();
            let end_str = &part[dash_pos + 1..].trim();

            if let (Ok(start), Ok(end)) = (start_str.parse::<u32>(), end_str.parse::<u32>()) {
                if start < max_layers && end < max_layers && start <= end {
                    layers.extend(start..=end);
                }
            }
        } else {
            // Single number
            if let Ok(layer) = part.parse::<u32>() {
                if layer < max_layers {
                    layers.push(layer);
                }
            }
        }
    }

    layers.sort_unstable();
    layers.dedup();

    if layers.is_empty() {
        None
    } else {
        Some(layers)
    }
}

pub fn find_missing_layers(assigned: &HashSet<u32>, total: u32) -> Vec<u32> {
    let mut missing = Vec::new();
    for i in 0..total {
        if !assigned.contains(&i) {
            missing.push(i);
        }
    }
    missing
}

pub fn determine_next_instances(
    assignments: &HashMap<String, Vec<u32>>,
) -> HashMap<String, String> {
    let mut next_instances = HashMap::new();

    // Create a map of first_layer -> shard
    let mut layer_to_shard: HashMap<u32, String> = HashMap::new();
    for (shard, layers) in assignments {
        if !layers.is_empty() {
            let min_layer = *layers.iter().min().unwrap();
            layer_to_shard.insert(min_layer, shard.clone());
        }
    }

    // For each shard, find its next shard
    for (shard, layers) in assignments {
        if !layers.is_empty() {
            let max_layer = *layers.iter().max().unwrap();

            // Find the shard that has max_layer + 1
            if let Some(next_shard) = layer_to_shard.get(&(max_layer + 1)) {
                next_instances.insert(shard.clone(), next_shard.clone());
            } else {
                // This is the last shard, connect back to the first
                if let Some(first_shard) = layer_to_shard.get(&0) {
                    next_instances.insert(shard.clone(), first_shard.clone());
                }
            }
        }
    }

    next_instances
}
