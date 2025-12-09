pub mod name_registry;
pub mod schema;
pub mod type_graph;

// TODO: write code to parse the display string of schema and type_graph into the respective types.
//          ideally keep it under #[cfg(test)]
// TODO: avoid redundant tests between schema.rs and type_graph.rs. have unified tests that test both modules together.
//          after asserting (json, schema) also assert (json, type_graph).
//          sometimes schema and type_graph have same forms (no recursive types / type duplicates)
//          and other times they differ.
