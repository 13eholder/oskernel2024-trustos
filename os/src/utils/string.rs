use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

#[inline(always)]
pub fn trim_start_slash(s: String) -> String {
    if s.chars().take_while(|c| *c == '/').count() >= 2 {
        format!("/{}", s.trim_start_matches('/'))
    } else {
        s
    }
}

pub fn path2abs<'a>(cwdv: &mut Vec<&'a str>, pathv: &Vec<&'a str>) -> String {
    for &path_element in pathv.iter() {
        if path_element == "." {
            continue;
        } else if path_element == ".." {
            cwdv.pop();
        } else {
            cwdv.push(path_element);
        }
    }
    let mut abs_path = String::from("/");
    abs_path.push_str(&cwdv.join("/"));
    abs_path
}

#[inline(always)]
pub fn path2vec(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

#[inline(always)]
pub fn is_abs_path(path: &str) -> bool {
    path.starts_with("/")
}
/// 用于路径拆分
pub fn rsplit_once<'a>(s: &'a str, delimiter: &str) -> (&'a str, &'a str) {
    let (mut parent_path, child_name) = s.rsplit_once(delimiter).unwrap();
    if parent_path.is_empty() {
        parent_path = "/";
    }
    (parent_path, child_name)
}

pub fn get_abs_path(base_path: &str, path: &str) -> String {
    if is_abs_path(&path) {
        path.to_string()
    } else {
        let mut wpath = {
            if base_path == "/" {
                Vec::with_capacity(32)
            } else {
                path2vec(base_path)
            }
        };
        path2abs(&mut wpath, &path2vec(&path))
    }
}

pub fn c_ptr_to_string(c_ptr: *const u8) -> String {
    let mut res = String::new();
    let mut i = 0;
    loop {
        let c = unsafe { *c_ptr.add(i) };
        if c == 0 {
            break;
        }
        res.push(c as char);
        i += 1;
    }
    res
}
