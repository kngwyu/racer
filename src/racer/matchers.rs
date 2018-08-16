use {scopes, typeinf, ast};
use core::{Match, PathSegment, Src, Session, Coordinate, SessionExt, BytePos, ByteRange};
use util::*;
use fileres::{get_crate_file, get_module_file};
use nameres::resolve_path;
use core::SearchType::{self, StartsWith, ExactMatch};
use core::MatchType::{self, Let, Module, Function, Struct, Type, Trait, Enum, EnumVariant,
                      Const, Static, IfLet, WhileLet, For, Macro};
use core::{self, Namespace, Visibility};
use std::path::Path;
use std::{str, vec};

/// The location of an import (`use` item) currently being resolved.
#[derive(PartialEq, Eq)]
struct PendingImport<'fp> {
    filepath: &'fp Path,
    range: ByteRange,
}

/// A stack of imports (`use` items) currently being resolved.
type PendingImports<'stack, 'fp> = StackLinkedListNode<'stack, PendingImport<'fp>>;

const GLOB_LIMIT: usize = 3;
/// Import information(pending imports, glob, and etc.)
pub struct ImportInfo<'stack, 'fp: 'stack> {
    /// A stack of imports currently being resolved
    imports: PendingImports<'stack, 'fp>,
    /// the max number of times where we can go through glob continuously
    /// if current search path isn't constructed via glob, it's none
    glob_limit: Option<usize>,
}

impl<'stack, 'fp: 'stack> Default for ImportInfo<'stack, 'fp> {
    fn default() -> Self {
        ImportInfo {
            imports: PendingImports::empty(),
            glob_limit: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchCxt<'s, 'p> {
    pub filepath: &'p Path,
    pub search_str: &'s str,
    pub range: ByteRange,
    pub search_type: SearchType,
    pub from_path: Option<core::Path>,
}

impl<'s, 'p> MatchCxt<'s, 'p> {
    fn get_key_ident(
        &self,
        blob: &str,
        keyword: &str,
        ignore: &[&str],
    ) -> Option<(BytePos, String, Visibility)> {
        find_keyword(blob, keyword, ignore, self).map(|start| {
            let s = match self.search_type {
                ExactMatch => self.search_str.to_owned(),
                StartsWith => {
                    let end = find_ident_end(blob, start + BytePos(self.search_str.len()));
                    blob[start.0..end.0].to_owned()
                }
            };
            (start, s)
        })
    }
}

pub fn match_types(
    src: Src,
    context: &MatchCxt,
    session: &Session,
    pending_imports: &ImportInfo
) -> impl Iterator<Item = Match> {
    match_extern_crate(&src, context, session).into_iter()
        .chain(match_mod(src, context, session))
        .chain(match_struct(&src, context))
        .chain(match_type(&src, context))
        .chain(match_trait(&src, context))
        .chain(match_enum(&src, context))
        .chain(match_use(&src, context, session, pending_imports))
}

pub fn match_values(src: Src, context: &MatchCxt) -> impl Iterator<Item=Match> {
    match_const(&src, context).into_iter()
        .chain(match_static(&src, context))
        .chain(match_fn(&src, context))
}

fn find_keyword(
    src: &str,
    pattern: &str,
    ignore: &[&str],
    context: &MatchCxt
) -> Option<(BytePos, Option<Visibility>)> {
    find_keyword_impl(
        src,
        pattern,
        context.search_str,
        ignore,
        context.search_type,
        &context.from_path,
    )
}

crate fn is_visible(vis: &Visibility, from_path: &core::Path) -> bool {
    true
}

fn find_keyword_impl(
    src: &str,
    pattern: &str,
    search_str: &str,
    ignore: &[&str],
    search_type: SearchType,
    from_path: &Option<core::Path>,
) -> Option<BytePos> {
    let mut start = BytePos::ZERO;
    let mut vis = None;
    if let Some((offset, vis_)) = strip_visivility(&src[..]) {
        start += offset;
        vis = Some(vis_);
    } else if !is_local {
        // TODO: too about
        return None;
    }

    if ignore.len() > 0 {
        start += strip_words(&src[start.0..], ignore);
    }
    // mandatory pattern\s+
    if !src[start.0..].starts_with(pattern) {
        return None;
    }
    // remove whitespaces ... must have one at least
    start += pattern.len().into();
    let oldstart = start;
    for &b in src[start.0..].as_bytes() {
        match b {
            b if is_whitespace_byte(b) => start = start.increment(),
            _ => break
        }
    }
    if start == oldstart { return None; }

    let search_str_len = search_str.len();
    if src[start.0..].starts_with(search_str) {
        match search_type {
            StartsWith => Some(start),
            ExactMatch => {
                if src.len() > start.0 + search_str_len &&
                    !is_ident_char(char_at(src, start.0 + search_str_len)) {
                    Some(start)
                } else {
                    None
                }
            }
        }
    } else {
        None
    }
}

fn is_const_fn(src: &str, blob_range: ByteRange) -> bool {
    if let Some(b) = strip_word(&src[blob_range.to_range()], "const") {
        let s = src[(blob_range.start + b).0..].trim_left();
        s.starts_with("fn") || s.starts_with("unsafe")
    } else {
        false
    }
}

fn match_pattern_start(
    src: &str,
    context: &MatchCxt,
    pattern: &str,
    ignore: &[&str],
    mtype: MatchType,
) -> Option<Match> {
    // ast currently doesn't contain the ident coords, so match them with a hacky
    // string search

    let blob = &src[context.range.to_range()];
    if let Some(start) = find_keyword(blob, pattern, ignore, context) {
        if let Some(end) = blob[start.0..].find(|c: char| c == ':' || c.is_whitespace()) {
            if blob[start.0 + end..].trim_left().chars().next() == Some(':') {
                let s = &blob[start.0..start.0 + end];
                return Some(Match {
                    matchstr: s.to_owned(),
                    filepath: context.filepath.to_path_buf(),
                    point: context.range.start + start,
                    coords: None,
                    local: context.is_local,
                    mtype: mtype,
                    contextstr: first_line(blob),
                    generic_args: Vec::new(),
                    generic_types: Vec::new(),
                    docs: String::new(),
                })
            }
        }
    }
    None
}

pub fn match_const(msrc: &str, context: &MatchCxt) -> Option<Match> {
    if is_const_fn(msrc, context.range) {
        return None;
    }
    // Here we don't have to ignore "unsafe"
    match_pattern_start(msrc, context, "const", &[], Const)
}

pub fn match_static(msrc: &str, context: &MatchCxt) -> Option<Match> {
    // Here we don't have to ignore "unsafe"
    match_pattern_start(msrc, context, "static", &[], Static)
}

fn match_pattern_let(msrc: &str, context: &MatchCxt, pattern: &str, mtype: MatchType) -> Vec<Match> {
    let mut out = Vec::new();
    let blob = &msrc[context.range.to_range()];
    if blob.starts_with(pattern) && txt_matches(context.search_type, context.search_str, blob) {
        let coords = ast::parse_pat_bind_stmt(blob.to_owned());
        for pat_range in coords {
            let s = &blob[pat_range.to_range()];
            if symbol_matches(context.search_type, context.search_str, s) {
                let start = context.range.start + pat_range.start;
                debug!("match_pattern_let point is {:?}", start);
                out.push(Match {
                    matchstr: s.to_owned(),
                    filepath: context.filepath.to_path_buf(),
                    point: start,
                    coords: None,
                    local: context.is_local,
                    mtype: mtype.clone(),
                    contextstr: first_line(blob),
                    generic_args: Vec::new(),
                    generic_types: Vec::new(),
                    docs: String::new(),
                });
                if context.search_type == ExactMatch {
                    break;
                }
            }
        }
    }
    out
}

pub fn match_if_let(msrc: &str, context: &MatchCxt) -> Vec<Match> {
    match_pattern_let(msrc, context, "if let ", IfLet)
}

pub fn match_while_let(msrc: &str, context: &MatchCxt) -> Vec<Match> {
    match_pattern_let(msrc, context, "while let ", WhileLet)
}

pub fn match_let(msrc: &str, context: &MatchCxt) -> Vec<Match> {
    match_pattern_let(msrc, context, "let ", Let)
}

pub fn match_for(msrc: &str, context: &MatchCxt) -> Vec<Match> {
    let mut out = Vec::new();
    let blob = &msrc[context.range.to_range()];
    let coords = ast::parse_pat_bind_stmt(blob.to_owned());
    for pat_range in coords {
        let s = &blob[pat_range.to_range()];
        if symbol_matches(context.search_type, context.search_str, s) {
            let start = pat_range.start + context.range.start;
            debug!("match_for point is {:?}, found ident {}", start, s);
            out.push(Match {
                matchstr: s.to_owned(),
                filepath: context.filepath.to_path_buf(),
                point: start,
                coords: None,
                visibility: Visibility::Local,
                mtype: For,
                contextstr: first_line(blob),
                generic_args: Vec::new(),
                generic_types: Vec::new(),
                docs: String::new(),
            });
        }
    }
    out
}

pub fn first_line(blob: &str) -> String {
    blob[..blob.find('\n').unwrap_or(blob.len())].to_owned()
}

/// Get the match's cleaned up context string
///
/// Strip all whitespace, including newlines in order to have a single line
/// context string.
pub fn get_context(blob: &str, context_end: &str) -> String {
    blob[..blob.find(context_end).unwrap_or(blob.len())]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn match_extern_crate(msrc: &str, context: &MatchCxt, session: &Session) -> Option<Match> {
    let mut res = None;
    let mut blob = &msrc[context.range.to_range()];
    let mut vis = Visibility::Crate;
    // Temporary fix to parse reexported crates by skipping pub
    // keyword until racer understands crate visibility.
    if let Some((offset, visibility)) = strip_visivility(blob) {
        blob = &blob[offset.0..];
        vis = visibility;
    }

    // TODO: later part is really necessary?
    if txt_matches(context.search_type, &format!("extern crate {}", context.search_str), blob) &&
        !(txt_matches(context.search_type, &format!("extern crate {} as", context.search_str), blob))
        || (blob.starts_with("extern crate") &&
            txt_matches(context.search_type, &format!("as {}", context.search_str), blob)) {

        debug!("found an extern crate: |{}|", blob);

        let extern_crate = ast::parse_extern_crate(blob.to_owned());

        if let Some(ref name) = extern_crate.name {
            debug!("extern crate {}", name);

            let realname = extern_crate.realname.as_ref().unwrap_or(name);
            if let Some(cratepath) = get_crate_file(realname, context.filepath, session) {
                let crate_src = session.load_file(&cratepath);
                res = Some(Match { matchstr: name.clone(),
                                   filepath: cratepath.to_path_buf(),
                                   point: BytePos::ZERO,
                                   coords: Some(Coordinate::start()),
                                   visibility: vis,
                                   mtype: Module,
                                   contextstr: cratepath.to_str().unwrap().to_owned(),
                                   generic_args: Vec::new(),
                                   generic_types: Vec::new(),
                                   docs: find_mod_doc(&crate_src, BytePos::ZERO),
                });
            }
        }
    }
    res
}

pub fn match_mod(msrc: Src, context: &MatchCxt, session: &Session) -> Option<Match> {
    let blob = &msrc[context.range.to_range()];
    let (start, s) = context.get_key_ident(blob, "mod", &[])?;
    if blob.find('{').is_some() {
        debug!("found a module inline: |{}|", blob);
        return Some(Match {
            matchstr: s,
            filepath: context.filepath.to_path_buf(),
            point: context.range.start + start,
            coords: None,
            local: false,
            mtype: Module,
            contextstr: context.filepath.to_str().unwrap().to_owned(),
            generic_args: Vec::new(),
            generic_types: Vec::new(),
            docs: String::new(),
        })
    } else {
        debug!("found a module declaration: |{}|", blob);

        let parent_path = context.filepath.parent()?;
        // get module from path attribute
        if let Some(modpath) = scopes::get_module_file_from_path(
            msrc,
            context.range.start,
            parent_path,
        ) {
            let msrc = session.load_file(&modpath);

            return Some(Match {
                matchstr: s,
                filepath: modpath.to_path_buf(),
                point: BytePos::ZERO,
                coords: Some(Coordinate::start()),
                local: false,
                mtype: Module,
                contextstr: modpath.to_str().unwrap().to_owned(),
                generic_args: Vec::new(),
                generic_types: Vec::new(),
                docs: find_mod_doc(&msrc, BytePos::ZERO),
            })
        }
        // get internal module nesting
        // e.g. is this in an inline submodule?  mod foo{ mod bar; }
        // because if it is then we need to search further down the
        // directory hierarchy - e.g. <cwd>/foo/bar.rs
        let internalpath = scopes::get_local_module_path(msrc, context.range.start);
        let mut searchdir = parent_path.to_owned();
        for s in internalpath {
            searchdir.push(&s);
        }
        if let Some(modpath) = get_module_file(&s, &searchdir, session) {
            let msrc = session.load_file(&modpath);
            let context = modpath.to_str().unwrap().to_owned();
            return Some(Match {
                matchstr: s,
                filepath: modpath,
                point: BytePos::ZERO,
                coords: Some(Coordinate::start()),
                local: false,
                mtype: Module,
                contextstr: context,
                generic_args: Vec::new(),
                generic_types: Vec::new(),
                docs: find_mod_doc(&msrc, BytePos::ZERO),
            })
        }
    }
    None
}

fn find_generics_end(blob: &str) -> Option<BytePos> {
    let mut level = 0;
    for (i, b) in blob.as_bytes().into_iter().enumerate() {
        match b {
            b'{' | b'(' | b';'  => return None,
            b'<' => level += 1,
            b'>' => {
                level -= 1;
                if level == 0 {
                    return Some(i.into())
                }
            }
            _ => {}
        }
    }
    None
}

pub fn match_struct(msrc: &str, context: &MatchCxt) -> Option<Match> {
    let blob = &msrc[context.range.to_range()];
    let (start, s) = context.get_key_ident(blob, "struct", &[])?;

    debug!("found a struct |{}|", s);

    let generics = if let Some(generics_end) = find_generics_end(&blob[start.0..]) {
        let header = format!("struct {}();", &blob[start.0..=(start + generics_end).0]);
        ast::parse_generics(header, context.filepath).get_idents()
    } else {
        Vec::new()
    };
    let start = context.range.start + start;
    Some(Match {
        matchstr: s,
        filepath: context.filepath.to_path_buf(),
        point: start,
        coords: None,
        local: context.is_local,
        mtype: Struct,
        contextstr: get_context(blob, "{"),
        generic_args: generics,
        generic_types: Vec::new(),
        docs: find_doc(msrc, start),
    })
}

pub fn match_type(msrc: &str, context: &MatchCxt) -> Option<Match> {
    let blob = &msrc[context.range.to_range()];
    let (start, s) = context.get_key_ident(blob, "type", &[])?;
    debug!("found!! a type {}", s);
    let start = context.range.start + start;
    Some(Match {
        matchstr: s,
        filepath: context.filepath.to_path_buf(),
        point: start,
        coords: None,
        local: context.is_local,
        mtype: Type,
        contextstr: first_line(blob),
        generic_args: Vec::new(),
        generic_types: Vec::new(),
        docs: find_doc(msrc, start),
    })
}

pub fn match_trait(msrc: &str, context: &MatchCxt) -> Option<Match> {
    let blob = &msrc[context.range.to_range()];
    let (start, s) = context.get_key_ident(blob, "trait", &[])?;
    debug!("found!! a trait {}", s);
    let start = context.range.start + start;
    Some(Match {
        matchstr: s,
        filepath: context.filepath.to_path_buf(),
        point: start,
        coords: None,
        local: context.is_local,
        mtype: Trait,
        contextstr: get_context(blob, "{"),
        generic_args: Vec::new(),
        generic_types: Vec::new(),
        docs: find_doc(msrc, start),
    })
}

pub fn match_enum_variants(msrc: &str, context: &MatchCxt) -> vec::IntoIter<Match> {
    let blob = &msrc[context.range.to_range()];
    let mut out = Vec::new();
    if (blob.starts_with("pub enum") || (context.is_local && blob.starts_with("enum"))) &&
       txt_matches(context.search_type, context.search_str, blob) {
        // parse the enum
        let parsed_enum = ast::parse_enum(blob.to_owned());

        for (name, offset) in parsed_enum.values {
            if name.starts_with(context.search_str) {
                let start = context.range.start + offset;
                let m = Match {
                    matchstr: name,
                    filepath: context.filepath.to_path_buf(),
                    point: start,
                    coords: None,
                    local: context.is_local,
                    mtype: EnumVariant(None),
                    contextstr: first_line(&blob[offset.0..]),
                    generic_args: Vec::new(),
                    generic_types: Vec::new(),
                    docs: find_doc(msrc, start),
                };
                out.push(m);
            }
        }
    }
    out.into_iter()
}

pub fn match_enum(msrc: &str, context: &MatchCxt) -> Option<Match> {
    let blob = &msrc[context.range.to_range()];
    let (start, s) = context.get_key_ident(blob, "enum", &[])?;

    debug!("found!! an enum |{}|", s);

    let generics = if let Some(generics_end) = find_generics_end(&blob[start.0..]) {
        let header = format!("enum {}{{}}", &blob[start.0..=(start + generics_end).0]);
        ast::parse_generics(header, context.filepath).get_idents()
    } else {
        Vec::new()
    };
    let start = context.range.start + start;
    Some(Match {
        matchstr: s,
        filepath: context.filepath.to_path_buf(),
        point: start,
        coords: None,
        local: context.is_local,
        mtype: Enum,
        contextstr: first_line(blob),
        generic_args: generics,
        generic_types: Vec::new(),
        docs: find_doc(msrc, start),
    })
}

pub fn match_use(
    msrc: &str,
    context: &MatchCxt,
    session: &Session,
    import_info: &ImportInfo,
) -> Vec<Match> {
    let import = PendingImport {
        filepath: context.filepath,
        range: context.range,
    };

    let blob = &msrc[context.range.to_range()];

    // If we're trying to resolve the same import recursively,
    // do not return any matches this time.
    if import_info.imports.contains(&import) {
        debug!("import {} involved in a cycle; ignoring", blob);
        return Vec::new();
    }

    // Push this import on the stack of pending imports.
    let pending_imports = import_info.imports.push(import);

    let mut out = Vec::new();

    if find_keyword_impl(blob, "use", "", &[], StartsWith, context.is_local).is_none() {
        return out;
    }

    let use_item = ast::parse_use(blob.to_owned());
    debug!(
        "[match_use] found item: {:?}, searchstr: {}",
        use_item, context.search_str
    );
    // for speed up!
    if !use_item.contains_glob && !txt_matches(context.search_type, context.search_str, blob) {
        return out;
    }
    let mut import_info = ImportInfo {
        imports: pending_imports,
        glob_limit: import_info.glob_limit,
    };
    // common utilities
    macro_rules! with_match {
        ($path:expr, $f:expr) => {
            let path_iter = resolve_path(
                $path,
                context.filepath,
                context.range.start,
                ExactMatch,
                Namespace::Both,
                session,
                &import_info,
            );
            for mut m in path_iter {
                $f(&mut m);
                out.push(m);
                if context.search_type == ExactMatch {
                    return out;
                }
            }
        };
    }
    // let's find searchstr using path_aliases
    for path_alias in use_item.path_list {
        match path_alias.kind {
            ast::PathAliasKind::Ident(ref ident) => {
                if !symbol_matches(context.search_type, context.search_str, ident) {
                    continue;
                }
                with_match!(path_alias.as_ref(), |m: &mut Match| {
                    debug!("[match_use] PathAliasKind::Ident {:?} was found", ident);
                    if m.matchstr != *ident {
                        m.matchstr = ident.clone();
                    }
                });
            }
            ast::PathAliasKind::Self_(ref ident) => {
                if let Some(last_seg) = path_alias.path.segments.last() {
                    let is_aliased = ident != "self";
                    let search_name = if is_aliased { ident } else { &last_seg.name };
                    if !symbol_matches(context.search_type, context.search_str, search_name) {
                        continue;
                    }
                    with_match!(path_alias.as_ref(), |m: &mut Match| {
                        debug!("[match_use] PathAliasKind::Self_ {:?} was found", ident);
                        if is_aliased && m.matchstr != *ident {
                            m.matchstr = ident.clone();
                        }
                    });
                }
            }
            ast::PathAliasKind::Glob => {
                let glob_depth_reserved = if let Some(ref mut d) = import_info.glob_limit {
                    if *d == 0 {
                        continue;
                    }
                    *d -= 1;
                    Some(*d + 1)
                } else {
                    // heuristics for issue #844
                    import_info.glob_limit = Some(GLOB_LIMIT - 1);
                    None
                };
                let mut search_path = path_alias.path;
                search_path.segments.push(PathSegment {
                    name: context.search_str.to_owned(),
                    types: vec![],
                });
                let mut path_iter = resolve_path(
                    &search_path,
                    context.filepath,
                    context.range.start,
                    context.search_type,
                    Namespace::Both,
                    session,
                    &import_info,
                );
                import_info.glob_limit = glob_depth_reserved;
                debug!(
                    "[match_use] resolve_path returned {:?} for Glob",
                    path_iter,
                );
                out.extend(path_iter);
            }
        }
    }
    out
}

pub fn match_fn(msrc: &str, context: &MatchCxt) -> Option<Match> {
    let blob = &msrc[context.range.to_range()];
    if typeinf::first_param_is_self(blob) {
        return None;
    }
    let (start, s) = context.get_key_ident(blob, "fn", &["const", "unsafe"])?;
    let start = context.range.start + start;
    Some(Match {
        matchstr: s,
        filepath: context.filepath.to_path_buf(),
        point: start,
        coords: None,
        local: context.is_local,
        mtype: Function,
        contextstr: get_context(blob, "{"),
        generic_args: Vec::new(),
        generic_types: Vec::new(),
        docs: find_doc(msrc, start),
    })
}

pub fn match_macro(msrc: &str, context: &MatchCxt) -> Option<Match> {
    let trimed = context.search_str.trim_right_matches('!');
    let mut context = context.clone();
    context.search_str = trimed;
    let blob = &msrc[context.range.to_range()];
    let (start, mut s) = context.get_key_ident(blob, "macro_rules!", &[])?;
    s.push('!');
    debug!("found a macro {}", s);
    Some(Match {
        matchstr: s,
        filepath: context.filepath.to_owned(),
        point: context.range.start + start,
        coords: None,
        local: context.is_local,
        mtype: Macro,
        contextstr: first_line(blob),
        generic_args: Vec::new(),
        generic_types: Vec::new(),
        docs: find_doc(msrc, context.range.start),
    })

}

pub fn find_doc(msrc: &str, match_point: BytePos) -> String {
    let blob = &msrc[0..match_point.0];
    blob.lines()
        .rev()
        .skip(1) // skip the line that the match is on
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("///") || line.starts_with("#[") || line.is_empty())
        .filter(|line| !(line.trim().starts_with("#[") || line.is_empty() ))  // remove the #[flags]
        .collect::<Vec<_>>()  // These are needed because
        .iter()               // you cannot `rev`an `iter` that
        .rev()                // has already been `rev`ed.
        .map(|line| if line.len() >= 4 { &line[4..] } else { "" })  // Remove "/// "
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_mod_doc(msrc: &str, blobstart: BytePos) -> String {
    let blob = &msrc[blobstart.0..];
    let mut doc = String::new();

    let mut iter = blob.lines()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("//") || line.is_empty())
        // Skip over the copyright notice and empty lines until you find
        // the module's documentation (it will go until the end of the
        // file if the module doesn't have any docs).
        .filter(|line| line.starts_with("//!"))
        .peekable();

    // Use a loop to avoid unnecessary collect and String allocation
    while let Some(line) = iter.next() {
        // Remove "//! " and push to doc string to be returned
        doc.push_str(if line.len() >= 4 { &line[4..] } else { "" });
        if iter.peek() != None {
            doc.push_str("\n");
        }
    }
    doc
}
