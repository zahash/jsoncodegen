pub mod name_registry;
pub mod schema;
pub mod type_graph;

// TODO: use indexmap to preserve insertion order or not? maybe keep them sorted
// TODO: write code to parse the display string of schema and type_graph into the respective types.
//          ideally keep it under #[cfg(test)]
// TODO: differentiate between Null and Unknown types.
//          right now both are combined under Unknown but behave like Null
