mod js;
mod python;
mod rust;
mod ts;

use super::LSPError;
use crate::{configs::FileType, lsp::client::Payload};
use serde_json::{from_str, Value};

fn paraser(text: Vec<String>) {}

struct Struct {
    name: String,
    parent: usize,
    attribute: Vec<String>,
    methods: Vec<String>,
}

impl Struct {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), parent: 0, methods: vec![], attribute: vec![] }
    }

    const fn parent(mut self, parent_id: usize) -> Self {
        self.parent = parent_id;
        self
    }

    fn attr(mut self, name: impl Into<String>) -> Self {
        self.attribute.push(name.into());
        self
    }

    fn meth(mut self, name: impl Into<String>) -> Self {
        self.methods.push(name.into());
        self
    }
}

struct Func {
    name: String,
    args: Vec<usize>,
    returns: Option<usize>,
}

struct Var {
    name: String,
    var_type: usize,
}

struct Defined {
    structs: Vec<Struct>,
    function: Vec<Func>,
    variables: Vec<Var>,
}

impl Defined {
    fn new() -> Self {
        Self {
            structs: vec![
                Struct::new("None"),
                Struct::new("tuple"),
                Struct::new("dict").meth("get").meth("remove").meth("keys").meth("items").meth("values"),
                Struct::new("list").meth("pop").meth("remove").meth("insert"),
                Struct::new("str"),
                Struct::new("int"),
                Struct::new("float"),
                Struct::new("bool"),
            ],
            function: vec![],
            variables: vec![
                Var { name: "True".to_owned(), var_type: 0 },
                Var { name: "False".to_owned(), var_type: 0 },
            ],
        }
    }

    fn struct_id(&mut self) -> usize {
        self.structs.len() + 1
    }
}

fn parse_file_open(data: String) -> Result<Vec<String>, LSPError> {
    if let Some((_header, msg)) = data.split_once("\r\n\r\n") {
        if let Some(text) = drill(from_str::<Value>(msg)?) {
            return Ok(text);
        }
    }
    Err(LSPError::internal("Expected file_did_open notification! Did not receive it ..."))
}

fn drill(mut val: Value) -> Option<Vec<String>> {
    let params = val.as_object_mut()?.get_mut("params")?;
    let documet = params.as_object_mut()?.get_mut("textDocument")?;
    let text = documet.as_object_mut()?.get("text")?.as_str()?;
    return Some(text.split('\n').map(ToOwned::to_owned).collect());
}
