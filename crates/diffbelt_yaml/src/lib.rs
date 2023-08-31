use std::mem::MaybeUninit;
use std::pin::Pin;
use std::slice::from_raw_parts;
use std::str::from_utf8;
use unsafe_libyaml::{
    yaml_document_delete, yaml_document_t, yaml_encoding_t, yaml_mark_t, yaml_node_item_t,
    yaml_node_pair_t, yaml_node_t, yaml_node_type_t, yaml_parser_delete, yaml_parser_initialize,
    yaml_parser_load, yaml_parser_set_encoding, yaml_parser_set_input_string, yaml_parser_t,
    yaml_stack_t,
};

#[derive(Debug)]
pub struct YamlMark {
    pub index: u64,
    pub line: u64,
    pub column: u64,
}

impl YamlMark {
    unsafe fn from_yaml_mark_t(value: *const yaml_mark_t) -> Self {
        let mark = &*value;

        Self {
            index: mark.index,
            line: mark.line,
            column: mark.column,
        }
    }
}

#[derive(Debug)]
pub enum YamlNodeValue {
    Empty,
    Scalar(YamlScalar),
    Sequence(YamlSequence),
    Mapping(YamlMapping),
}

#[derive(Debug)]
pub struct YamlScalar {
    pub value: String,
}

#[derive(Debug)]
pub struct YamlSequence {
    pub items: Vec<YamlNode>,
}

impl YamlSequence {
    unsafe fn from_yaml_stack_t(
        root_stack: *const yaml_stack_t<yaml_node_t>,
        stack: *const yaml_stack_t<yaml_node_item_t>,
    ) -> Result<Self, ()> {
        let mut items = Vec::new();

        let root_stack = &*root_stack;
        let stack = &*stack;

        if stack.top == stack.start {
            return Ok(Self { items });
        }

        let mut node_ptr = stack.start;

        while node_ptr != stack.top {
            let index: i32 = *node_ptr;

            let node = root_stack.start.add((index - 1) as usize);
            let node = YamlNode::from_yaml_node_t(root_stack, node)?;

            items.push(node);

            node_ptr = node_ptr.add(1);
        }

        Ok(Self { items })
    }
}

#[derive(Debug)]
pub struct YamlMapping {
    pub items: Vec<(YamlNode, YamlNode)>,
}

impl YamlMapping {
    unsafe fn from_yaml_stack_t(
        root_stack: *const yaml_stack_t<yaml_node_t>,
        stack: *const yaml_stack_t<yaml_node_pair_t>,
    ) -> Result<Self, ()> {
        let mut items = Vec::new();

        let root_stack = &*root_stack;
        let stack = &*stack;

        if stack.top == stack.start {
            return Ok(Self { items });
        }

        let mut node_ptr = stack.start;

        while node_ptr != stack.top {
            let pair = &*node_ptr;
            let key_index: i32 = pair.key;
            let value_index: i32 = pair.value;

            let key_node = root_stack.start.add((key_index - 1) as usize);
            let key = YamlNode::from_yaml_node_t(root_stack, key_node)?;

            let value_node = root_stack.start.add((value_index - 1) as usize);
            let value = YamlNode::from_yaml_node_t(root_stack, value_node)?;

            items.push((key, value));

            node_ptr = node_ptr.add(1);
        }

        Ok(Self { items })
    }
}

#[derive(Debug)]
pub struct YamlNode {
    pub value: YamlNodeValue,
    pub start_mark: YamlMark,
}

impl YamlNode {
    unsafe fn from_yaml_node_t(
        root_stack: *const yaml_stack_t<yaml_node_t>,
        node: *const yaml_node_t,
    ) -> Result<Self, ()> {
        let node = &*node;
        let start_mark = YamlMark::from_yaml_mark_t(&node.start_mark);

        let value = match node.type_ {
            yaml_node_type_t::YAML_NO_NODE => YamlNodeValue::Empty,
            yaml_node_type_t::YAML_SCALAR_NODE => {
                let s = from_raw_parts(
                    node.data.scalar.value,
                    node.data.scalar.length.try_into().unwrap(),
                );
                let s = from_utf8(s).map_err(|_| ())?;

                YamlNodeValue::Scalar(YamlScalar {
                    value: s.to_string(),
                })
            }
            yaml_node_type_t::YAML_SEQUENCE_NODE => YamlNodeValue::Sequence(
                YamlSequence::from_yaml_stack_t(root_stack, &node.data.sequence.items)?,
            ),
            yaml_node_type_t::YAML_MAPPING_NODE => YamlNodeValue::Mapping(
                YamlMapping::from_yaml_stack_t(root_stack, &node.data.mapping.pairs)?,
            ),
            _ => {
                return Err(());
            }
        };

        Ok(Self { value, start_mark })
    }
}

struct Parser {
    value: Pin<Box<MaybeUninit<yaml_parser_t>>>,
}

impl Parser {
    fn ptr(&mut self) -> *mut yaml_parser_t {
        self.value.as_mut_ptr()
    }

    fn new(input: &str) -> Result<Self, ()> {
        unsafe {
            let parser_value = MaybeUninit::uninit();
            let mut parser_value = Box::pin(parser_value);
            let parser = parser_value.as_mut_ptr();
            let result = yaml_parser_initialize(parser);
            if !result.ok {
                return Err(());
            }

            let input = input.as_bytes();
            let input_ptr = input.as_ptr();
            let input_size = input.len() as u64;

            yaml_parser_set_encoding(parser, yaml_encoding_t::YAML_UTF8_ENCODING);
            yaml_parser_set_input_string(parser, input_ptr, input_size);

            Ok(Self {
                value: parser_value,
            })
        }
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        unsafe {
            yaml_parser_delete(self.ptr());
        }
    }
}

struct Document {
    value: Pin<Box<MaybeUninit<yaml_document_t>>>,
    is_initialized: bool,
}

impl Document {
    fn new() -> Self {
        Self {
            value: Box::pin(MaybeUninit::uninit()),
            is_initialized: false,
        }
    }

    fn ptr(&mut self) -> *mut yaml_document_t {
        self.value.as_mut_ptr()
    }

    fn load(&mut self, parser: &mut Parser) -> Result<(), ()> {
        unsafe {
            if self.is_initialized {
                yaml_document_delete(self.ptr());
            }

            let result = yaml_parser_load(parser.ptr(), self.ptr());

            if result.ok {
                self.is_initialized = true;
                Ok(())
            } else {
                Err(())
            }
        }
    }

    unsafe fn empty(&mut self) -> bool {
        let doc = &*self.ptr();

        doc.nodes.start == doc.nodes.top
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        if self.is_initialized {
            unsafe {
                yaml_document_delete(self.ptr());
            }
        }
    }
}

pub fn parse_yaml(yaml: &str) -> Result<Vec<YamlNode>, ()> {
    let mut nodes = Vec::new();

    unsafe {
        let mut parser = Parser::new(yaml)?;
        let mut document = Document::new();

        loop {
            document.load(&mut parser)?;

            if document.empty() {
                return Ok(nodes);
            }

            let doc = &*document.ptr();

            let node = YamlNode::from_yaml_node_t(&doc.nodes, doc.nodes.start)?;

            nodes.push(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_yaml;

    #[test]
    fn parse_cli_config() {
        let config = r#"
anchored: &test
  value: 42
  list:
    - with_values: yes
    - and_lists: [1, 'test', "something", 42]
      tratata: wut
with_anchor: *test
"#;

        let docs = parse_yaml(config).expect("parsed");

        println!("{:?}", docs);
    }
}
