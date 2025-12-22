pub mod name_registry;
pub mod schema;
pub mod type_graph;

// #[cfg(test)]
// mod tests {
//     use crate::{schema::Schema, type_graph::TypeGraph};
//     use pretty_assertions::assert_eq;

//     #[test]
//     fn empty_structures() {
//         TestCase::json("{}")
//             .schema("{}")
//             .rootless_type_graph("{}")
//             .run();

//         TestCase::json("[]")
//             .schema("[unknown]")
//             .rootless_type_graph("[unknown]")
//             .run();

//         TestCase::json("[null]")
//             .schema("[null]")
//             .rootless_type_graph("[null]")
//             .run();
//     }

//     #[test]
//     fn single_primitive_arrays() {
//         TestCase::json("[true]")
//             .schema("[bool]")
//             .rootless_type_graph("[bool]")
//             .run();

//         TestCase::json("[123]")
//             .schema("[int]")
//             .rootless_type_graph("[int]")
//             .run();

//         TestCase::json("[123.5]")
//             .schema("[float]")
//             .rootless_type_graph("[float]")
//             .run();

//         TestCase::json(r#"["s"]"#)
//             .schema("[str]")
//             .rootless_type_graph("[str]")
//             .run();
//     }

//     #[test]
//     fn union() {
//         TestCase::json("[1, 2.5]")
//             .schema("[|int|float|]")
//             .rootless_type_graph("[|int|float|]")
//             .run();

//         TestCase::json(r#"["a", 5]"#)
//             .schema("[|int|str|]")
//             .rootless_type_graph("[|int|str|]")
//             .run();

//         TestCase::json(r#"["s", {"a":1}]"#)
//             .schema("[|str|{a:int}|]")
//             .rootless_type_graph("[|str|{a:int}|]")
//             .run();

//         TestCase::json(r#"[{"a":1}, [1]]"#)
//             .schema("[|[int]|{a:int}|]")
//             .rootless_type_graph("[|[int]|{a:int}|]")
//             .run();
//     }

//     #[test]
//     fn null() {
//         TestCase::json("[null, null]")
//             .schema("[null]")
//             .rootless_type_graph("[null]")
//             .run();
//     }

//     #[test]
//     fn empty_array_unknown() {
//         TestCase::json("[[], [1,2]]")
//             .schema("[[int]]")
//             .rootless_type_graph("[[int]]")
//             .run();

//         TestCase::json(r#"{"x": []}"#)
//             .schema("{x:[unknown]}")
//             .rootless_type_graph("{x:[unknown]}")
//             .run();
//     }

//     #[test]
//     fn optional() {
//         TestCase::json("[null, 5]")
//             .schema("[int?]")
//             .rootless_type_graph("[int?]")
//             .run();

//         TestCase::json("[5, null]")
//             .schema("[int?]")
//             .rootless_type_graph("[int?]")
//             .run();

//         TestCase::json("[null, true]")
//             .schema("[bool?]")
//             .rootless_type_graph("[bool?]")
//             .run();

//         TestCase::json(r#"[null, "hello"]"#)
//             .schema("[str?]")
//             .rootless_type_graph("[str?]")
//             .run();

//         TestCase::json("[null, null, 5]")
//             .schema("[int?]")
//             .rootless_type_graph("[int?]")
//             .run();

//         TestCase::json("[[1], null]")
//             .schema("[[int]?]")
//             .rootless_type_graph("[[int]?]")
//             .run();

//         TestCase::json("[null, [1]]")
//             .schema("[[int]?]")
//             .rootless_type_graph("[[int]?]")
//             .run();
//     }

//     #[test]
//     fn optional_union() {
//         TestCase::json("[2.2, 1, null]")
//             .schema("[|int|float|?]")
//             .rootless_type_graph("[|int|float|?]")
//             .run();

//         TestCase::json(r#"["s", 1, null]"#)
//             .schema("[|int|str|?]")
//             .rootless_type_graph("[|int|str|?]")
//             .run();
//     }

//     #[test]
//     fn nested_arrays() {
//         TestCase::json("[[1], [2]]")
//             .schema("[[int]]")
//             .rootless_type_graph("[[int]]")
//             .run();

//         TestCase::json(r#"[[1], ["a"]]"#)
//             .schema("[[|int|str|]]")
//             .rootless_type_graph("[[|int|str|]]")
//             .run();
//     }

//     #[test]
//     fn array_of_objects_with_disjoint_fields() {
//         TestCase::json(r#"[{"a":1}, {}]"#)
//             .schema("[{a:int?}]")
//             .rootless_type_graph("[{a:int?}]")
//             .run();

//         TestCase::json(r#"[{"a":1}, {"b":"x"}]"#)
//             .schema("[{a:int?,b:str?}]")
//             .rootless_type_graph("[{a:int?,b:str?}]")
//             .run();

//         TestCase::json(r#"[{"a":1}, {"a":2, "b":"x"}, {"c":3.14, "a":2}]"#)
//             .schema("[{a:int,b:str?,c:float?}]")
//             .rootless_type_graph("[{a:int,b:str?,c:float?}]")
//             .run();
//     }

//     #[test]
//     fn mixed_nesting() {
//         TestCase::json(
//             r#"
//             [
//                 {"a": [{"b": [1, 2]}]},
//                 {"a": [{"b": [3]}]}
//             ]
//             "#,
//         )
//         .schema("[{a:[{b:[int]}]}]")
//         .rootless_type_graph("[{a:[{b:[int]}]}]")
//         .run();
//     }

//     #[test]
//     fn object() {
//         TestCase::json(r#"{"x": 1}"#)
//             .schema("{x:int}")
//             .rootless_type_graph("{x:int}")
//             .run();

//         TestCase::json(r#"{"x": null}"#)
//             .schema("{x:null}")
//             .rootless_type_graph("{x:null}")
//             .run();

//         TestCase::json(r#"{"x": [1,2]}"#)
//             .schema("{x:[int]}")
//             .rootless_type_graph("{x:[int]}")
//             .run();

//         TestCase::json(r#"{"x": ["a", 1, null]}"#)
//             .schema("{x:[|int|str|?]}")
//             .rootless_type_graph("{x:[|int|str|?]}")
//             .run();

//         TestCase::json(r#"{"a": {"b": {"c": {"d": {"e": 1}}}}}"#)
//             .schema("{a:{b:{c:{d:{e:int}}}}}")
//             .rootless_type_graph("{a:{b:{c:{d:{e:int}}}}}")
//             .run();
//     }

//     #[test]
//     fn ecommerce_api_response() {
//         TestCase::json(
//             r#"
//             {
//                 "user": {
//                     "id": 123,
//                     "name": "Alice",
//                     "email": "alice@example.com",
//                     "verified": true,
//                     "address": {
//                         "city": "London",
//                         "zip": 40512
//                     }
//                 },
//                 "cart": [
//                     {
//                         "sku": "SKU-123",
//                         "qty": 2,
//                         "price": 499.99,
//                         "metadata": null
//                     },
//                     {
//                         "sku": "SKU-999",
//                         "qty": 1,
//                         "price": 1299.50,
//                         "metadata": { "color": "red" }
//                     }
//                 ],
//                 "payment": null,
//                 "discount_codes": ["HOLIDAY", 2024, null]
//             }
//             "#,
//         )
//         .schema(
//             "{\
//             cart:[{metadata:{color:str}?,price:float,qty:int,sku:str}],\
//             discount_codes:[|int|str|?],\
//             payment:null,\
//             user:{address:{city:str,zip:int},email:str,id:int,name:str,verified:bool}\
//         }",
//         )
//         .rootless_type_graph(
//             "{\
//             cart:[{metadata:{color:str}?,price:float,qty:int,sku:str}],\
//             discount_codes:[|int|str|?],\
//             payment:null,\
//             user:{address:{city:str,zip:int},email:str,id:int,name:str,verified:bool}\
//         }",
//         )
//         .run();
//     }

//     #[test]
//     fn config_file() {
//         TestCase::json(
//             r#"
//             {
//                 "version": "1.0",
//                 "services": [
//                     {"name": "db", "replicas": 2, "env": ["POSTGRES=1", "DEBUG=true"]},
//                     {"name": "api", "replicas": 3, "env": null},
//                     {"name": "ui", "replicas": 1},
//                     {"name": "cache", "replicas": 1, "env": ["REDIS=1"]}
//                 ],
//                 "features": {
//                     "auth": true,
//                     "logging": { "level": "debug", "files": ["a.log", "b.log"] },
//                     "metrics": null
//                 }
//             }
//             "#,
//         )
//         .schema(
//             "{\
//                 features:{auth:bool,logging:{files:[str],level:str},metrics:null},\
//                 services:[{env:[str]?,name:str,replicas:int}],\
//                 version:str\
//             }",
//         )
//         .rootless_type_graph(
//             "{\
//                 features:{auth:bool,logging:{files:[str],level:str},metrics:null},\
//                 services:[{env:[str]?,name:str,replicas:int}],\
//                 version:str\
//             }",
//         )
//         .run();
//     }

//     #[test]
//     fn analytics_events() {
//         TestCase::json(
//             r#"
//             [
//                 {"event":"click", "x":10, "y":20},
//                 {"event":"scroll", "delta": 300},
//                 {"event":"purchase", "amount": 129.99, "currency":"USD"},
//                 {"event":"click", "x":5, "y":10, "timestamp":"2025-01-01T12:00Z"}
//             ]
//             "#,
//         )
//         .schema(
//             "[{\
//                 amount:float?,\
//                 currency:str?,\
//                 delta:int?,\
//                 event:str,\
//                 timestamp:str?,\
//                 x:int?,\
//                 y:int?\
//             }]",
//         )
//         .rootless_type_graph(
//             "[{\
//                 amount:float?,\
//                 currency:str?,\
//                 delta:int?,\
//                 event:str,\
//                 timestamp:str?,\
//                 x:int?,\
//                 y:int?\
//             }]",
//         )
//         .run();
//     }

//     #[test]
//     fn linked_list() {
//         // Type graph reduces recursive structures, but schema shows full nesting
//         TestCase::json(
//             r#"
//             [
//                 { "val": 1, "next": null, "prev": null },
//                 { "val": 1, "next": { "val": 2, "next": null, "prev": null }, "prev": null },
//                 { "val": 1, "next": null, "prev": { "val": 2, "next": null, "prev": null } }
//             ]
//             "#,
//         )
//         .schema(
//             "[{next:{next:null,prev:null,val:int}?,prev:{next:null,prev:null,val:int}?,val:int}]",
//         )
//         .rootless_type_graph("[{next:next?,prev:next?,val:int}]")
//         .run();

//         TestCase::json(
//             r#"
//             {
//                 "val": 1,
//                 "prev": {"val": 2, "prev": null, "next": null},
//                 "next": {
//                     "val": 3,
//                     "prev": null,
//                     "next": {"val": 4, "prev": null, "next": null}
//                 }
//             }
//             "#,
//         )
//         .schema("{next:{next:{next:null,prev:null,val:int},prev:null,val:int},prev:{next:null,prev:null,val:int},val:int}")
//         .full_type_graph("next;{next:next?,prev:next?,val:int}")
//         .run();
//     }

//     #[test]
//     fn tree() {
//         TestCase::json(
//             r#"
//             {
//                 "name": "Root",
//                 "children": [
//                     {
//                         "name": "Child1",
//                         "children": []
//                     },
//                     {
//                         "name": "Child2",
//                         "children": [
//                             {
//                                 "name": "Grandchild",
//                                 "children": []
//                             }
//                         ]
//                     }
//                 ]
//             }
//             "#,
//         )
//         .schema("{children:[{children:[{children:[unknown],name:str}],name:str}],name:str}")
//         .rootless_type_graph("children;{children:[children],name:str}")
//         .run();
//     }

//     struct TestCase<'a> {
//         json: &'a str,
//         schema: Option<&'a str>,
//         rootless_type_graph: Option<&'a str>,
//         full_type_graph: Option<&'a str>,
//     }

//     impl<'a> TestCase<'a> {
//         fn json(json: &'a str) -> Self {
//             Self {
//                 json,
//                 schema: None,
//                 rootless_type_graph: None,
//                 full_type_graph: None,
//             }
//         }

//         fn schema(mut self, schema: &'a str) -> Self {
//             self.schema = Some(schema);
//             self
//         }

//         fn rootless_type_graph(mut self, rootless_type_graph: &'a str) -> Self {
//             self.rootless_type_graph = Some(rootless_type_graph);
//             self
//         }

//         fn full_type_graph(mut self, full_type_graph: &'a str) -> Self {
//             self.full_type_graph = Some(full_type_graph);
//             self
//         }

//         #[track_caller]
//         fn run(self) {
//             let json = serde_json::from_str::<serde_json::Value>(self.json).expect("invalid json");

//             if let Some(schema) = self.schema {
//                 assert_eq!(format!("{}", Schema::from(json.clone())), schema, "SCHEMA");
//             }

//             if let Some(rootless_type_graph) = self.rootless_type_graph {
//                 assert_eq!(
//                     format!("{}", TypeGraph::from(json.clone()))
//                         .split_once(';')
//                         .expect("expected root to be separated by ; delimiter")
//                         .1,
//                     rootless_type_graph,
//                     "ROOTLESS TYPE GRAPH"
//                 );
//             }

//             if let Some(full_type_graph) = self.full_type_graph {
//                 assert_eq!(
//                     format!("{}", TypeGraph::from(json)),
//                     full_type_graph,
//                     "FULL TYPE GRAPH"
//                 );
//             }
//         }
//     }
// }
