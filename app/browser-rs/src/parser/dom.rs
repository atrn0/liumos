//! This is a part of "13.2.6 Tree construction" in the HTML spec.
//! https://html.spec.whatwg.org/multipage/parsing.html#tree-construction

use crate::parser::tokenizer::*;
#[allow(unused_imports)]
use liumlib::*;

use alloc::rc::{Rc, Weak};
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

#[allow(dead_code)]
#[derive(Debug, Clone)]
/// https://dom.spec.whatwg.org/#interface-node
pub struct Node {
    pub kind: NodeKind,
    pub parent: Option<Weak<RefCell<Node>>>,
    pub first_child: Option<Rc<RefCell<Node>>>,
    pub last_child: Option<Weak<RefCell<Node>>>,
    pub previous_sibling: Option<Weak<RefCell<Node>>>,
    pub next_sibling: Option<Rc<RefCell<Node>>>,
}

#[allow(dead_code)]
///dom.spec.whatwg.org/#interface-node
impl Node {
    pub fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            parent: None,
            first_child: None,
            last_child: None,
            previous_sibling: None,
            next_sibling: None,
        }
    }

    pub fn first_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.first_child.as_ref().map(|n| n.clone())
    }

    pub fn last_child(&self) -> Option<Weak<RefCell<Node>>> {
        self.last_child.as_ref().map(|n| n.clone())
    }

    pub fn previous_sibling(&self) -> Option<Weak<RefCell<Node>>> {
        self.previous_sibling.as_ref().map(|n| n.clone())
    }

    pub fn next_sibling(&self) -> Option<Rc<RefCell<Node>>> {
        self.next_sibling.as_ref().map(|n| n.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// https://dom.spec.whatwg.org/#interface-document
    Document,
    /// https://dom.spec.whatwg.org/#interface-element
    Element(Element),
    /// https://dom.spec.whatwg.org/#interface-text
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// https://dom.spec.whatwg.org/#interface-element
pub struct Element {
    kind: ElementKind,
    //id: String,
    //class_name: String,
}

impl Element {
    pub fn new(kind: ElementKind) -> Self {
        Self {
            kind,
            //id: String::new(),
            //class_name: String::new(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// https://dom.spec.whatwg.org/#interface-element
pub enum ElementKind {
    /// https://html.spec.whatwg.org/multipage/semantics.html#the-html-element
    Html,
    /// https://html.spec.whatwg.org/multipage/semantics.html#the-head-element
    Head,
    /// https://html.spec.whatwg.org/multipage/sections.html#the-body-element
    Body,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    AfterBody,
    AfterAfterBody,
}

#[derive(Debug, Clone)]
pub struct Parser {
    root: Rc<RefCell<Node>>,
    mode: InsertionMode,
    t: Tokenizer,
    /// https://html.spec.whatwg.org/multipage/parsing.html#the-stack-of-open-elements
    stack_of_open_elements: Vec<Rc<RefCell<Node>>>,
}

impl Parser {
    pub fn new(t: Tokenizer) -> Self {
        Self {
            root: Rc::new(RefCell::new(Node::new(NodeKind::Document))),
            mode: InsertionMode::Initial,
            t,
            stack_of_open_elements: Vec::new(),
        }
    }

    /// Creates an element node.
    fn create_element(&self, kind: ElementKind) -> Node {
        return Node::new(NodeKind::Element(Element::new(kind)));
    }

    /// Creates a char node.
    fn create_char(&self, c: char) -> Node {
        let mut s = String::new();
        s.push(c);
        return Node::new(NodeKind::Text(s));
    }

    /// Creates an element based on the `tag` string.
    fn create_element_by_tag(&self, tag: &str) -> Node {
        if tag == "html" {
            return self.create_element(ElementKind::Html);
        } else if tag == "head" {
            return self.create_element(ElementKind::Head);
        } else if tag == "body" {
            return self.create_element(ElementKind::Body);
        }
        panic!("not supported this tag name: {}", tag);
    }

    /// Creates an element node for the token and insert it to the appropriate place for inserting
    /// a node. Put the new node in the stack of open elements.
    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-foreign-element
    fn insert_element(&mut self, tag: &str) {
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n,
            None => &self.root,
        };

        let node = Rc::new(RefCell::new(self.create_element_by_tag(tag)));

        if current.borrow().first_child().is_some() {
            {
                current
                    .borrow()
                    .first_child()
                    .unwrap()
                    .borrow_mut()
                    .next_sibling = Some(node.clone());
            }
            {
                node.borrow_mut().previous_sibling =
                    Some(Rc::downgrade(&current.borrow().first_child().unwrap()));
            }
        } else {
            current.borrow_mut().first_child = Some(node.clone());
        }

        {
            current.borrow_mut().last_child = Some(Rc::downgrade(&node));
        }
        {
            node.borrow_mut().parent = Some(Rc::downgrade(&current));
        }

        self.stack_of_open_elements.push(node);
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-character
    fn insert_char(&mut self, c: char) {
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n,
            None => &self.root,
        };

        {
            match current.borrow_mut().kind {
                NodeKind::Text(ref mut s) => {
                    s.push(c);
                    return;
                }
                _ => {}
            }
        }

        let node = Rc::new(RefCell::new(self.create_char(c)));

        if current.borrow().first_child().is_some() {
            {
                current
                    .borrow()
                    .first_child()
                    .unwrap()
                    .borrow_mut()
                    .next_sibling = Some(node.clone());
            }
            {
                node.borrow_mut().previous_sibling =
                    Some(Rc::downgrade(&current.borrow().first_child().unwrap()));
            }
        } else {
            current.borrow_mut().first_child = Some(node.clone());
        }

        {
            current.borrow_mut().last_child = Some(Rc::downgrade(&node));
        }
        {
            node.borrow_mut().parent = Some(Rc::downgrade(&current));
        }

        self.stack_of_open_elements.push(node);
    }

    /// Returns true if the current node's kind is same as NodeKind::Element::<element_kind>.
    fn pop_current_node(&mut self, element_kind: ElementKind) -> bool {
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n,
            None => return false,
        };

        if current.borrow().kind == NodeKind::Element(Element::new(element_kind)) {
            self.stack_of_open_elements.pop();
            return true;
        }

        false
    }

    /// Pops nodes until a node with `element_kind` comes.
    fn pop_until(&mut self, element_kind: ElementKind) {
        assert!(self.contain_in_stack(element_kind));

        loop {
            let current = match self.stack_of_open_elements.pop() {
                Some(n) => n,
                None => return,
            };

            if current.borrow().kind == NodeKind::Element(Element::new(element_kind)) {
                return;
            }
        }
    }

    /// Returns true if the stack of open elements has NodeKind::Element::<element_kind> node.
    fn contain_in_stack(&mut self, element_kind: ElementKind) -> bool {
        for i in 0..self.stack_of_open_elements.len() {
            if self.stack_of_open_elements[i].borrow().kind
                == NodeKind::Element(Element::new(element_kind))
            {
                return true;
            }
        }

        false
    }

    pub fn construct_tree(&mut self) -> Rc<RefCell<Node>> {
        let mut token = self.t.next();

        while token.is_some() {
            match self.mode {
                // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
                InsertionMode::Initial => self.mode = InsertionMode::BeforeHtml,

                // https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
                InsertionMode::BeforeHtml => {
                    match token {
                        Some(Token::Doctype) => {
                            token = self.t.next();
                            continue;
                        }
                        Some(Token::Char(c)) => {
                            // If a character token that is one of U+0009 CHARACTER TABULATION, U+000A
                            // LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or
                            // U+0020 SPACE, ignore the token.
                            let num = c as u32;
                            if num == 0x09
                                || num == 0x0a
                                || num == 0x0c
                                || num == 0x0d
                                || num == 0x20
                            {
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(Token::StartTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            // A start tag whose tag name is "html"
                            // Create an element for the token in the HTML namespace, with the Document
                            // as the intended parent. Append it to the Document object. Put this
                            // element in the stack of open elements.
                            if tag == "html" {
                                self.insert_element(tag);
                                self.mode = InsertionMode::BeforeHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(Token::EndTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            // Any other end tag
                            // Parse error. Ignore the token.
                            if tag != "head" || tag != "body" || tag != "html" || tag != "br" {
                                // Ignore the token.
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(Token::Eof) | None => {
                            return self.root.clone();
                        }
                    }
                    self.insert_element("html");
                    self.mode = InsertionMode::BeforeHead;
                } // end of InsertionMode::BeforeHtml

                // https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
                InsertionMode::BeforeHead => {
                    match token {
                        Some(Token::Char(c)) => {
                            let num = c as u32;
                            // If a character token that is one of U+0009 CHARACTER TABULATION, U+000A
                            // LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or
                            // U+0020 SPACE, ignore the token.
                            if num == 0x09
                                || num == 0x0a
                                || num == 0x0c
                                || num == 0x0d
                                || num == 0x20
                            {
                                token = self.t.next();
                            }
                        }
                        Some(Token::StartTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            if tag == "head" {
                                self.insert_element(tag);
                                self.mode = InsertionMode::InHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(Token::Eof) | None => {
                            return self.root.clone();
                        }
                        _ => {}
                    }
                    self.insert_element("head");
                    self.mode = InsertionMode::InHead;
                } // end of InsertionMode::BeforeHead

                // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
                InsertionMode::InHead => {
                    match token {
                        Some(Token::EndTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            if tag == "head" {
                                self.mode = InsertionMode::AfterHead;
                                token = self.t.next();
                                assert!(self.pop_current_node(ElementKind::Head));
                                continue;
                            }
                        }
                        Some(Token::Eof) | None => {
                            return self.root.clone();
                        }
                        _ => {}
                    }
                    self.mode = InsertionMode::AfterHead;
                    assert!(self.pop_current_node(ElementKind::Head));
                } // end of InsertionMode::InHead

                // https://html.spec.whatwg.org/multipage/parsing.html#the-after-head-insertion-mode
                InsertionMode::AfterHead => {
                    match token {
                        Some(Token::StartTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            if tag == "body" {
                                self.insert_element(tag);
                                token = self.t.next();
                                self.mode = InsertionMode::InBody;
                                continue;
                            }
                        }
                        Some(Token::Eof) | None => {
                            return self.root.clone();
                        }
                        _ => {}
                    }
                    self.insert_element("body");
                    self.mode = InsertionMode::InBody;
                } // end of InsertionMode::AfterHead

                // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody
                InsertionMode::InBody => {
                    match token {
                        Some(Token::StartTag {
                            tag: _,
                            self_closing: _,
                        }) => {}
                        Some(Token::EndTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            if tag == "body" {
                                self.mode = InsertionMode::AfterBody;
                                token = self.t.next();
                                if !self.contain_in_stack(ElementKind::Body) {
                                    // Parse error. Ignore the token.
                                    continue;
                                }
                                self.pop_until(ElementKind::Body);
                                continue;
                            }
                            if tag == "html" {
                                // If the stack of open elements does not have a body element in
                                // scope, this is a parse error; ignore the token.
                                if self.pop_current_node(ElementKind::Body) {
                                    self.mode = InsertionMode::AfterBody;
                                    assert!(self.pop_current_node(ElementKind::Html));
                                } else {
                                    token = self.t.next();
                                }
                                continue;
                            }
                        }
                        Some(Token::Char(c)) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }
                        Some(Token::Eof) | None => {
                            return self.root.clone();
                        }
                        _ => {}
                    }
                } // end of InsertionMode::InBody

                // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
                InsertionMode::AfterBody => {
                    match token {
                        Some(Token::EndTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            if tag == "html" {
                                self.mode = InsertionMode::AfterAfterBody;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(Token::Eof) | None => {
                            return self.root.clone();
                        }
                        _ => {}
                    }

                    self.mode = InsertionMode::InBody;
                } // end of InsertionMode::AfterBody

                // https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
                InsertionMode::AfterAfterBody => {
                    match token {
                        Some(Token::EndTag {
                            ref tag,
                            self_closing: _,
                        }) => {
                            if tag == "html" {
                                self.mode = InsertionMode::AfterAfterBody;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(Token::Eof) | None => {
                            return self.root.clone();
                        }
                        _ => {}
                    }

                    self.mode = InsertionMode::InBody;
                } // end of InsertionMode::AfterAfterBody
            } // end of match self.mode {}
        } // end of while token.is_some {}

        self.root.clone()
    }
}
