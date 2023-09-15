pub mod node_helpers;
pub mod serde;

pub use crate::serde::decode_yaml;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::slice::from_raw_parts;
use std::str::from_utf8;
use unsafe_libyaml::{
    yaml_document_delete, yaml_document_t, yaml_encoding_t, yaml_mark_t, yaml_node_item_t,
    yaml_node_pair_t, yaml_node_t, yaml_node_type_t, yaml_parser_delete, yaml_parser_initialize,
    yaml_parser_load, yaml_parser_set_encoding, yaml_parser_set_input_string, yaml_parser_t,
    yaml_stack_t,
};

#[derive(Debug)]
pub enum YamlParsingError {
    InitializationFailed,
    UnknownNodeTypeAt(YamlMark),
    NotUtf8At(YamlMark),
    LoopDetected,
    // TODO: use streaming parsed variant, show error position
    Parsing,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct YamlMark {
    pub index: u64,
    pub line: u64,
    pub column: u64,
}

impl YamlMark {
    pub fn empty() -> Self {
        Self {
            index: 0,
            line: 0,
            column: 0,
        }
    }

    unsafe fn from_yaml_mark_t(value: *const yaml_mark_t) -> Self {
        let mark = &*value;

        Self {
            index: mark.index,
            line: mark.line + 1,
            column: mark.column + 1,
        }
    }
}

impl AsRef<YamlMark> for &YamlMark {
    fn as_ref(&self) -> &YamlMark {
        self
    }
}
impl AsRef<YamlMark> for &YamlNode {
    fn as_ref(&self) -> &YamlMark {
        &self.start_mark
    }
}
impl AsRef<YamlMark> for &Rc<YamlNode> {
    fn as_ref(&self) -> &YamlMark {
        &self.start_mark
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum YamlNodeValue {
    Empty,
    Scalar(YamlScalar),
    Sequence(YamlSequence),
    Mapping(YamlMapping),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct YamlScalar {
    pub value: Rc<str>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct YamlSequence {
    pub items: Rc<Vec<Rc<YamlNode>>>,
}

struct ParsingState {
    used_nodes: Vec<bool>,
}

impl YamlSequence {
    unsafe fn from_yaml_stack_t(
        state: &mut ParsingState,
        root_stack: *const yaml_stack_t<yaml_node_t>,
        stack: *const yaml_stack_t<yaml_node_item_t>,
    ) -> Result<Self, YamlParsingError> {
        let mut items = Vec::new();

        let root_stack = &*root_stack;
        let stack = &*stack;

        if stack.top == stack.start {
            return Ok(Self {
                items: Rc::new(items),
            });
        }

        let mut node_ptr = stack.start;

        while node_ptr != stack.top {
            let index = (*node_ptr - 1) as usize;

            let node = root_stack.start.add(index);
            let node = YamlNode::from_yaml_node_t(state, root_stack, node, index)?;

            items.push(Rc::new(node));

            node_ptr = node_ptr.add(1);
        }

        Ok(Self {
            items: Rc::new(items),
        })
    }
}

impl<'a> IntoIterator for &'a YamlSequence {
    type Item = &'a YamlNode;
    type IntoIter =
        std::iter::Map<std::slice::Iter<'a, Rc<YamlNode>>, fn(&'a Rc<YamlNode>) -> &'a YamlNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter().map(|node| node.deref())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct YamlMapping {
    pub items: Vec<(Rc<YamlNode>, Rc<YamlNode>)>,
}

impl YamlMapping {
    unsafe fn from_yaml_stack_t(
        state: &mut ParsingState,
        root_stack: *const yaml_stack_t<yaml_node_t>,
        stack: *const yaml_stack_t<yaml_node_pair_t>,
    ) -> Result<Self, YamlParsingError> {
        let mut items = Vec::new();

        let root_stack = &*root_stack;
        let stack = &*stack;

        if stack.top == stack.start {
            return Ok(Self { items });
        }

        let mut node_ptr = stack.start;

        while node_ptr != stack.top {
            let pair = &*node_ptr;
            let key_index = (pair.key - 1) as usize;
            let value_index = (pair.value - 1) as usize;

            let key_node = root_stack.start.add(key_index);
            let key = YamlNode::from_yaml_node_t(state, root_stack, key_node, key_index)?;

            let value_node = root_stack.start.add(value_index);
            let value = YamlNode::from_yaml_node_t(state, root_stack, value_node, value_index)?;

            items.push((Rc::new(key), Rc::new(value)));

            node_ptr = node_ptr.add(1);
        }

        Ok(Self { items })
    }
}

impl<'a> IntoIterator for &'a YamlMapping {
    type Item = (&'a YamlNode, &'a YamlNode);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (Rc<YamlNode>, Rc<YamlNode>)>,
        fn(&'a (Rc<YamlNode>, Rc<YamlNode>)) -> (&'a YamlNode, &'a YamlNode),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.items
            .iter()
            .map(|(key, value)| (key.deref(), value.deref()))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct YamlNode {
    pub value: YamlNodeValue,
    pub tag: Option<Rc<str>>,
    pub start_mark: YamlMark,
}

#[derive(Debug)]
pub struct YamlNodeRc(Rc<YamlNode>);

impl Deref for YamlNodeRc {
    type Target = Rc<YamlNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl YamlNode {
    unsafe fn from_yaml_node_t(
        state: &mut ParsingState,
        root_stack: *const yaml_stack_t<yaml_node_t>,
        node: *const yaml_node_t,
        node_index: usize,
    ) -> Result<Self, YamlParsingError> {
        let node = &*node;
        let start_mark = YamlMark::from_yaml_mark_t(&node.start_mark);

        let mut is_used_node = false;

        let tag = if *node.tag == '!' as u8 {
            let tag: &CStr = CStr::from_ptr(node.tag as *const i8);
            let Ok(tag) = tag.to_str() else {
                return Err(YamlParsingError::NotUtf8At(start_mark));
            };

            Some(Rc::from(tag))
        } else {
            None
        };

        let value = match node.type_ {
            yaml_node_type_t::YAML_NO_NODE => YamlNodeValue::Empty,
            yaml_node_type_t::YAML_SCALAR_NODE => {
                let s = from_raw_parts(
                    node.data.scalar.value,
                    node.data.scalar.length.try_into().unwrap(),
                );
                let Ok(s) = from_utf8(s) else {
                    return Err(YamlParsingError::NotUtf8At(start_mark));
                };

                YamlNodeValue::Scalar(YamlScalar { value: Rc::from(s) })
            }
            yaml_node_type_t::YAML_SEQUENCE_NODE => {
                if state.used_nodes[node_index] {
                    return Err(YamlParsingError::LoopDetected);
                }

                is_used_node = true;
                state.used_nodes[node_index] = true;

                YamlNodeValue::Sequence(YamlSequence::from_yaml_stack_t(
                    state,
                    root_stack,
                    &node.data.sequence.items,
                )?)
            }
            yaml_node_type_t::YAML_MAPPING_NODE => {
                if state.used_nodes[node_index] {
                    return Err(YamlParsingError::LoopDetected);
                }

                is_used_node = true;
                state.used_nodes[node_index] = true;

                YamlNodeValue::Mapping(YamlMapping::from_yaml_stack_t(
                    state,
                    root_stack,
                    &node.data.mapping.pairs,
                )?)
            }
            _ => {
                return Err(YamlParsingError::UnknownNodeTypeAt(start_mark));
            }
        };

        if is_used_node {
            state.used_nodes[node_index] = false;
        }

        Ok(Self {
            value,
            tag,
            start_mark,
        })
    }
}

struct Parser {
    value: Pin<Box<MaybeUninit<yaml_parser_t>>>,
}

impl Parser {
    fn ptr(&mut self) -> *mut yaml_parser_t {
        self.value.as_mut_ptr()
    }

    fn new(input: &str) -> Result<Self, YamlParsingError> {
        unsafe {
            let parser_value = MaybeUninit::uninit();
            let mut parser_value = Box::pin(parser_value);
            let parser = parser_value.as_mut_ptr();
            let result = yaml_parser_initialize(parser);
            if !result.ok {
                return Err(YamlParsingError::InitializationFailed);
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

    fn load(&mut self, parser: &mut Parser) -> Result<(), YamlParsingError> {
        unsafe {
            if self.is_initialized {
                yaml_document_delete(self.ptr());
            }

            let result = yaml_parser_load(parser.ptr(), self.ptr());

            if result.ok {
                self.is_initialized = true;
                Ok(())
            } else {
                Err(YamlParsingError::Parsing)
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

pub fn parse_yaml(yaml: &str) -> Result<Vec<YamlNode>, YamlParsingError> {
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

            let max_nodes_count = doc.nodes.end.offset_from(doc.nodes.start) as usize;

            let mut state = ParsingState {
                used_nodes: vec![false; max_nodes_count],
            };

            let node = YamlNode::from_yaml_node_t(&mut state, &doc.nodes, doc.nodes.start, 0)?;

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
  list: !tagged
    - with_values: yes
    - and_lists: [1, 'test', "something", 42]
      tratata: wuts
with_anchor: *test
"#;

        let docs = parse_yaml(config).expect("parsed");

        println!("{:?}", docs);
    }
}
