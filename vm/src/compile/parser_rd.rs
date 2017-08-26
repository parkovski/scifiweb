use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::env;
use std::fmt::Debug;
use nom::IResult;
use fxhash::FxHashSet;
use ast::*;
use util::split_vec::SplitVec;
use super::lexer;
use super::parse_errors::*;

use super::token::*;
/// Get the value from inside the TokenKind.
macro_rules! extract {
  ($kind:ident, $token:expr) => (
    if let TokenKind::$kind(v) = $token {
      v
    } else {
      panic!("Tried to extract {} from {}", stringify!($kind), &$token)
    }
  );
  (&$kind:ident, $token:expr) => (
    if let &TokenKind::$kind(v) = $token {
      v
    } else {
      panic!("Tried to extract {} from {}", stringify!($kind), $token)
    }
  )
}

/// Optionally parse something.
/// Transforms unexpected token errors to None,
/// leaves other errors as errors.
fn optional<T>(res: Result<T>) -> Result<Option<T>> {
  match res {
    Ok(t) => Ok(Some(t)),
    Err(Error(ErrorKind::UnexpectedToken(_), _)) => Ok(None),
    Err(Error(ErrorKind::Expected(_), _)) => Ok(None),
    Err(e) => Err(e),
  }
}

pub struct Parser<'a> {
  filename: Arc<PathBuf>,
  token: Token<'a>,
  included_paths: &'a mut FxHashSet<Arc<PathBuf>>,
  inp: &'a [u8],
  ast: Ast,
}

impl<'a> Parser<'a> {
  fn new(
    filename: Arc<PathBuf>,
    included_paths: &'a mut FxHashSet<Arc<PathBuf>>,
    inp: &'a [u8]
  )
    -> Self
  {
    let token_span = TokenSpan::new(filename.clone());
    let token = Token::new(TokenKind::Invalid('\0'), token_span);
    Parser {
      filename,
      token,
      included_paths,
      inp,
      ast: Ast::new(),
    }
  }

  fn include(&mut self, filename: &str) -> Result<()> {
    let filename = Arc::new(
      match self.filename.parent() {
        Some(parent) if !parent.as_os_str().is_empty()
          => parent.join(filename),
        _ => env::current_dir().expect("Unknown current dir").join(filename),
      }
      .canonicalize()?
    );
    
    if !self.included_paths.insert(filename.clone()) {
      trace!("Skipping already included file '{}'", filename.to_string_lossy());
      return Ok(());
    }
    let sub_ast = Self::parse_file(filename, &mut self.included_paths)?;
    self.ast.merge(sub_ast);
    Ok(())
  }

  pub fn parse(filename: &Path) -> Result<Ast> {
    let mut includes: FxHashSet<_> = Default::default();
    let filename = Arc::new(filename.canonicalize()?);
    includes.insert(filename.clone());
    Self::parse_file(filename, &mut includes)
  }

  fn parse_file(filename: Arc<PathBuf>, includes: &mut FxHashSet<Arc<PathBuf>>)
    -> Result<Ast>
  {
    let mut program = String::new();
    File::open(filename.as_ref())?.read_to_string(&mut program)?;
    trace!("Loading {}", filename.to_string_lossy());
    let mut parser = Parser::new(filename, includes, program.as_bytes());
    parser.parse_program()?;
    Ok(parser.into_ast())
  }

  pub fn parse_str(filename: PathBuf, program: &str) -> Result<Ast> {
    let mut includes = Default::default();
    let mut parser = Parser::new(Arc::new(filename), &mut includes, program.as_bytes());
    parser.parse_program()?;
    Ok(parser.into_ast())
  }

  fn into_ast(self) -> Ast {
    self.ast
  }

  fn advance(&mut self) -> Result<()> {
    let (token, inp) = match lexer::next_token(self.inp, &self.token.span) {
      IResult::Done(inp, token) => (token, inp),
      IResult::Incomplete(_) => unreachable!("Lexer should not return incomplete"),
      IResult::Error(e) => return Err(Error::from_nom(e, &self.token.span)),
    };
    self.inp = inp;
    self.token = token;
    Ok(())
  }

  /// Top level: Include | Label: (def keyword)
  fn parse_program(&mut self) -> Result<()> {
    self.advance()?;
    loop {
      if self.token == TokenKind::Eof {
        return Ok(())
      } else if self.token == Keyword::Include {
        self.parse_include()?
      } else if let TokenKind::Label(label) = self.token.kind {
        self.advance()?;
        // An item definition
        let token = self.expect(TokenMatch::Keyword)?;
        let keyword = extract!(Keyword, token.kind);
        match keyword {
          Keyword::Collectable => self.parse_collectable_or_group(label)?,
          Keyword::User => self.parse_user(label)?,
          Keyword::Event => self.parse_event(label)?,
          _ => return self.e_unexpected(),
        }
      } else {
        return self.e_unexpected();
      }
    }
  }

  // ===== Include =====

  fn parse_include(&mut self) -> Result<()> {
    self.consume(Keyword::Include)?;
    let path = self.take(TokenMatch::String)?;
    self.consume(TokenKind::Semicolon)?;
    self.include(extract!(String, path.kind))?;
    Ok(())
  }

  // ===== User =====

  fn parse_user(&mut self, label: &str) -> Result<()> {
    self.consume(Keyword::User)?;
    self.consume(TokenKind::Semicolon)?;
    Ok(())
  }

  // ===== Collectable =====

  fn parse_collectable_or_group(&mut self, label: &str) -> Result<()> {
    self.consume(Keyword::Collectable)?;
    let is_group = self.opt_consume(Keyword::Group)?;
    self.consume(TokenKind::Semicolon)?;
    let mut group = CollectableGroup::new(label.into());
    // Subdefs starting with 'has'
    let subdefs_has = [
      Self::parse_has_collectable,
      Self::parse_has_upgrades,
      Self::parse_has_redemptions,
    ];
    if self.opt_consume(Keyword::Has)? {
      // First one can be 'has amount', after that only the above group.
      if self.opt_consume(Keyword::Amount)? {
        group.has_amount = true;
      } else {
        self.any(&subdefs_has, &mut group, true)?;
      }
      self.consume(TokenKind::Semicolon)?;
    }
    while self.opt_consume(Keyword::Has)? {
      self.any(&subdefs_has, &mut group, true)?;
      self.consume(TokenKind::Semicolon)?;
    }
    Ok(())
  }

  /// Ident | Label (upgrades, redemptions)?
  fn parse_inline_collectable(&mut self, group: &mut CollectableGroup) -> Result<()> {
    if self.token == TokenMatch::Identifier {
      self.advance()?;
    } else if self.token == TokenMatch::Label {
      self.advance()?;
      self.all(&[Self::parse_has_upgrades, Self::parse_has_redemptions], group, false)?;
    }
    Ok(())
  }

  fn parse_inline_collectable_group(&mut self, group: &mut CollectableGroup) -> Result<()> {
    self.consume(TokenMatch::Identifier)?;
    Ok(())
  }

  fn parse_has_collectable(&mut self, group: &mut CollectableGroup) -> Result<()> {
    self.consume(Keyword::Collectable)?;
    let parser = if self.opt_consume(Keyword::Group)? {
      Self::parse_inline_collectable_group
    } else {
      Self::parse_inline_collectable
    };
    self.parse_single_or_list(parser, group)?;
    Ok(())
  }

  fn parse_has_upgrades(&mut self, container: &mut CollectableGroup) -> Result<()> {
    self.consume(Keyword::Upgrades)?;
    Ok(())
  }

  fn parse_has_redemptions(&mut self, container: &mut CollectableGroup) -> Result<()> {
    self.consume(Keyword::Redemptions)?;
    Ok(())
  }

  // ===== Event =====

  fn parse_event(&mut self, label: &str) -> Result<()> {
    self.consume(Keyword::Event)?;
    Ok(())
  }

  // ===== Variables =====

  // ===== General =====

  fn parse_single_or_list<F, P, I>(&mut self, mut inner: F, param: &mut P) -> Result<Vec<I>>
  where
    F: FnMut(&mut Self, &mut P) -> Result<I>,
  {
    if self.token == TokenKind::LSquareBracket {
      self.parse_bracketed_list(inner, param)
    } else {
      Ok(vec![inner(self, param)?])
    }
  }

  fn parse_bracketed_list<F, P, I>(&mut self, mut inner: F, param: &mut P) -> Result<Vec<I>>
  where
    F: FnMut(&mut Self, &mut P) -> Result<I>,
  {
    self.consume(TokenKind::LSquareBracket)?;
    let mut items = Vec::new();
    loop {
      if let Some(item) = optional(inner(self, param))? {
        items.push(item);
        if self.opt_consume(TokenKind::Comma)? {
          continue;
        }
      }
      if self.opt_consume(TokenKind::RSquareBracket)? {
        break;
      }
      return self.e_expected("']'");
    }
    Ok(items)
  }

  // ===== Helpers =====

  fn e_expected<T: Debug, O>(&self, t: T) -> Result<O> {
    Err(ErrorKind::Expected(format!("expected {:?}, found {}", t, &self.token)).into())
  }

  fn e_unexpected<O>(&self) -> Result<O> {
    Err(ErrorKind::UnexpectedToken(self.token.to_string()).into())
  }

  /// Move to the next token, returning the current if it matches.
  fn take<T: PartialEq<Token<'a>> + Debug>(&mut self, t: T) -> Result<Token<'a>> {
    if &t == &self.token {
      let token = self.token.clone();
      self.advance()?;
      Ok(token)
    } else {
      self.e_expected(t)
    }
  }

  /// Return the current token if it matches, otherwise error. Don't advance.
  fn expect<T: PartialEq<Token<'a>> + Debug>(&mut self, t: T) -> Result<Token<'a>> {
    if &t == &self.token {
      Ok(self.token.clone())
    } else {
      self.e_expected(t)
    }
  }

  /// Move to the next token if the current matches, otherwise error.
  fn consume<T: PartialEq<Token<'a>> + Debug>(&mut self, t: T) -> Result<()> {
    if &t == &self.token {
      self.advance()?;
      Ok(())
    } else {
      self.e_expected(t)
    }
  }

  /// Move to the next token if the current matches, otherwise false.
  fn opt_consume<T: PartialEq<Token<'a>> + Debug>(&mut self, t: T) -> Result<bool> {
    if &t == &self.token {
      self.advance()?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  /// Provided functions must decide without taking
  /// any extra tokens.
  fn any<'b, I, P>(&'b mut self, fns: I, param: &'b mut P, required: bool) -> Result<bool>
  where
    I: IntoIterator<Item = &'b fn(&mut Self, &mut P) -> Result<()>> + 'b,
    P: 'b,
  {
    for f in fns {
      if let Some(()) = optional(f(self, param))? {
        return Ok(true);
      }
    }
    if required {
      self.e_unexpected()
    } else {
      Ok(false)
    }
  }

  /// Runs all, but not in any particular order.
  fn all<'b, I, P>(&'b mut self, fns: I, param: &'b mut P, required: bool) -> Result<bool>
  where
    I: IntoIterator<Item = &'b fn(&mut Self, &mut P) -> Result<()>> + 'b,
    P: 'b,
  {
    // Done parsers on left, pending on right.
    let mut fns: SplitVec<_> = fns.into_iter().collect();
    let mut right_len = fns.right_len();
    'try_next: while right_len > 0 {
      let split = fns.split_index();
      for i in split..(split + right_len) {
        let f = fns[i];
        if optional(f(self, param))?.is_some() {
          fns.move_left(i);
          right_len = fns.right_len();
          continue 'try_next;
        }
      }
      return if required {
        self.e_unexpected()
      } else {
        Ok(false)
      };
    }
    Ok(true)
  }
}
