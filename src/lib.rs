extern crate itertools;
extern crate classfile_parser;
#[macro_use]
extern crate quote;

extern crate proc_macro2;

use std::io;
use std::io::{Read, Write};

use std::process::{Command, Stdio, Child, ChildStdin, ChildStdout};

use itertools::Itertools;

use classfile_parser::*;
use classfile_parser::constant_info::*;
use proc_macro2::{Ident, Span, TokenStream};
use std::borrow::Cow;

trait ClassFileExt {
    fn str_from_pool(&self, ind: u16) -> Result<&str, ()>;
}

impl ClassFileExt for ClassFile {
    fn str_from_pool(&self, ind: u16) -> Result<&str, ()> {
        match self.const_pool[ind as usize - 1] {
            ConstantInfo::Utf8(ref utf8_const) => Ok(&utf8_const.utf8_string),
            _ => Err(())
        }
    }
}


fn get_class_name<'a>(class: &'a ClassConstant, class_file: &'a ClassFile) -> &'a str {
    class_file.str_from_pool(class.name_index).unwrap()
}

struct RustFmt(Child);

impl RustFmt {
    fn launch() -> io::Result<RustFmt> {
        let cmd = Command::new("rustfmt")
            .stdin(Stdio::piped())
            //.stdout(Stdio::piped())
            .spawn()?;
        Ok(RustFmt(cmd))
    }

    fn stdin(&mut self) -> &mut ChildStdin { self.0.stdin.as_mut().unwrap() }
    fn stdout(&mut self) -> &mut ChildStdout { self.0.stdout.as_mut().unwrap() }
}

fn transform_name(java_name: &str) -> String {
    if java_name == "<init>" { return String::from("new"); }
    if java_name == "<clinit>" { unimplemented!() }

    let mut res = String::new();
    let mut first = true;
    for (is_upper, chars) in &java_name.chars().group_by(|c| c.is_uppercase()) {
        let chars: Vec<char> = chars.collect();

        if chars.len() == 1 {
            if first || chars[0].is_lowercase() {
                res.push(chars[0]);
                res.push('_');
            } else {
                res.push(chars[0].to_ascii_lowercase());
            }
        } else if chars[0].is_lowercase() {
            res.extend(chars);
            res.push('_');
        } else {
            let last_index = chars.len() - 1;
            res.extend(&chars[..last_index]);
            res.push('_');
            res.push(chars[last_index].to_ascii_lowercase());
        }

        first = false;
    }
    res.pop(); // strip the trailing underscore
    res
}

fn gen_method(class_file: &ClassFile, method: &method_info::MethodInfo) -> TokenStream {
    let name = class_file.str_from_pool(method.name_index).unwrap();
    let ident = Ident::new(&transform_name(name), Span::call_site());
    quote! {
        fn #ident(&self) {
            // TODO
        }
    }
}

fn bindgen(class_name: &str) {
    let classfile = parse_class(class_name)
        .expect("Failed to parse the class");

    let class = match &classfile.const_pool[classfile.this_class as usize - 1] {
        ConstantInfo::Class(c) => c,
        _ => panic!("Malformed classfile, expected this_class to point to a class"),
    };

    let name = get_class_name(class, &classfile);
    let ident = Ident::new(name, proc_macro2::Span::call_site());

    let mut methods = TokenStream::new();
    for method in &classfile.methods {
        methods.extend(gen_method(&classfile, method));
    }

    let gen: proc_macro2::TokenStream = quote! {
        struct #ident<'a>(JObject<'a>);

        impl<'a> #ident<'a> {
            #methods
        }
    };

    let mut rustfmt = RustFmt::launch().expect("Failed to launch rustfmt");
    write!(rustfmt.stdin(), "{}", gen);

}

#[cfg(test)]
mod test {
    #[test]
    fn it_works() {
        super::bindgen("/tmp/HelloWorld")
    }
}
