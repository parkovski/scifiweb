use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::env;
use nom::IResult;
use fxhash::FxHashSet;
use ast::*;
use ast::ty::*;
use util::split_vec::SplitVec;
use util::graph_cell::*;
use super::lexer;
use super::parse_errors::*;

use super::token::*;
/// Get the value from inside the TokenKind.
macro_rules! extract {
  ($self_:ident, $kind:ident in $token:expr) => (
    if let TokenKind::$kind(v) = $token.kind {
      Ok(v)
    } else {
      $self_.e_expected(stringify!($kind))
    }
  );
  ($self_:ident, $kind:ident) => (
    extract!($self_, $kind in $self_.token)
  );
}

/// Optionally parse something.
/// Transforms unexpected token errors to None,
/// leaves other errors as errors.
fn optional<T>(res: Result<T>) -> Result<Option<T>> {
  match res {
    Ok(t) => Ok(Some(t)),
    Err(Error(ErrorKind::Unexpected(..), _)) => Ok(None),
    Err(Error(ErrorKind::Expected(..), _)) => Ok(None),
    Err(e) => Err(e),
  }
}

pub struct Parser<'p, 'ast: 'p> {
  filename: Arc<PathBuf>,
  token: Token<'p>,
  included_paths: &'p mut FxHashSet<Arc<PathBuf>>,
  inp: &'p [u8],
  ast: GraphRefMut<'ast, Ast<'ast>>,
}

impl<'p, 'ast: 'p> Parser<'p, 'ast> {
  fn new(
    filename: Arc<PathBuf>,
    included_paths: &'p mut FxHashSet<Arc<PathBuf>>,
    inp: &'p [u8],
    ast: GraphRefMut<'ast, Ast<'ast>>,
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
      ast,
    }
  }

  fn string_token_value(&self) -> TokenValue<Arc<str>> {
    let kind = self.token.kind.as_str();
    let ss = self.ast.awake_ref().shared_string(kind);
    let span = self.token.span.clone();
    TokenValue::new(ss, span)
  }

  fn include(
    &mut self,
    filename: &str,
  ) -> Result<()>
  {
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
    Self::parse_file(filename, &mut self.included_paths, self.ast.clone())
  }

  pub fn parse(filename: &Path) -> Result<Box<GraphCell<Ast<'ast>>>> {
    let mut includes: FxHashSet<_> = Default::default();
    let filename = Arc::new(filename.canonicalize()?);
    includes.insert(filename.clone());
    let ast = Ast::new();
    Self::parse_file(filename, &mut includes, ast.asleep_mut())?;
    ast.awake().typecheck()?;
    Ok(ast)
  }

  fn parse_file(
    filename: Arc<PathBuf>,
    includes: &'p mut FxHashSet<Arc<PathBuf>>,
    ast: GraphRefMut<'ast, Ast<'ast>>,
  ) -> Result<()>
  {
    let mut program = String::new();
    File::open(filename.as_ref())?.read_to_string(&mut program)?;
    trace!("Loading {}", filename.to_string_lossy());
    let mut parser = Self::new(filename, includes, program.as_bytes(), ast);
    parser.parse_program()
  }

  pub fn parse_str(
    filename: PathBuf,
    program: &'p str,
  ) -> Result<Box<GraphCell<Ast<'ast>>>>
  {
    let mut includes = Default::default();
    let ast = Ast::new();
    let mut parser = Self::new(
      Arc::new(filename),
      &mut includes,
      program.as_bytes(),
      ast.asleep_mut(),
    );
    parser.parse_program()?;
    ast.awake().typecheck()?;
    Ok(ast)
  }

  fn lexer_iresult(&self) -> Result<(Token<'p>, &'p [u8])> {
    match lexer::next_token(self.inp, &self.token.span) {
      IResult::Done(inp, token) => Ok((token, inp)),
      IResult::Incomplete(_) => unreachable!("Lexer should not return incomplete"),
      IResult::Error(e) => Err(Error::from_nom(e, &self.token.span)),
    }
  }

  fn advance(&mut self) -> Result<()> {
    let (token, inp) = self.lexer_iresult()?;
    self.inp = inp;
    self.token = token;
    Ok(())
  }

  fn peek(&self) -> Result<Token> {
    let (token, _) = self.lexer_iresult()?;
    Ok(token)
  }

  /// Top level = Include | Def block
  /// Def block = ident <def keyword> (';' | ':' body 'end;')
  fn parse_program(&mut self) -> Result<()> {
    self.advance()?;
    loop {
      if self.token == TokenKind::Eof {
        return Ok(())
      } else if self.token == Keyword::Include {
        self.parse_include()?
      } else if let TokenKind::Identifier(label) = self.token.kind {
        let label = self.string_token_value();
        self.advance()?;
        // An item (type) definition
        let base_type = self.parse_base_custom_type()?;
        if self.opt_consume(TokenKind::Semicolon)? {
          // Empty item
          self.ast.awake_mut().insert(base_type.into_empty_type(label))?;
        } else {
          self.consume(TokenKind::Colon)?;
          let ty = match base_type {
            | BaseCustomType::Collectable
            | BaseCustomType::CollectableGroup
              => self.parse_collectable_or_group(label, base_type),
            | BaseCustomType::User => self.parse_user(label),
            | BaseCustomType::Event => self.parse_event(label),
            | _ => unimplemented!(),
          }?;
          self.consume(Keyword::End)?;
          self.consume(TokenKind::Semicolon)?;
          self.ast.awake_mut().insert(ty);
        }
      } else {
        return self.e_unexpected();
      }
    }
  }

  fn parse_base_custom_type(&mut self) -> Result<BaseCustomType> {
    self.expect(TokenMatch::Keyword)?;
    let kwd = extract!(self, Keyword)?;
    Ok(match kwd {
      Keyword::Collectable => {
        self.advance()?;
        if self.opt_consume(Keyword::Group)? {
          BaseCustomType::CollectableGroup
        } else {
          BaseCustomType::Collectable
        }
      }
      Keyword::User => {
        self.advance()?;
        if self.opt_consume(Keyword::Group)? {
          BaseCustomType::UserGroup
        } else {
          BaseCustomType::User
        }
      }
      Keyword::Remote => {
        self.advance()?;
        let kwd = self.take(TokenMatch::Keyword)?;
        if self.opt_consume(Keyword::Event)? {
          BaseCustomType::RemoteEvent
        } else if self.opt_consume(Keyword::Function)? {
          BaseCustomType::RemoteFunction
        } else {
          return self.e_expected("event or function");
        }
      }
      Keyword::Array => {
        self.advance()?;
        BaseCustomType::Array
      }
      Keyword::Object => {
        self.advance()?;
        BaseCustomType::Object
      }
      Keyword::Event => {
        self.advance()?;
        BaseCustomType::Event
      }
      Keyword::Function => {
        self.advance()?;
        BaseCustomType::Function
      }
      _ => return self.e_expected("base type keyword"),
    })
  }

  // ===== Include =====

  fn parse_include(&mut self) -> Result<()> {
    self.consume(Keyword::Include)?;
    let path_token = self.take(TokenMatch::String)?;
    self.consume(TokenKind::Semicolon)?;
    let path = extract!(self, String in path_token)?;
    self.include(path)?;
    Ok(())
  }

  // ===== User =====

  fn parse_user(&mut self, label: TokenValue<Arc<str>>) -> Result<Type<'ast>> {
    self.consume(Keyword::User)?;
    self.consume(TokenKind::Semicolon)?;
    unimplemented!()
  }

  // ===== Collectable =====

  fn parse_collectable_or_group(
    &mut self,
    label: TokenValue<Arc<str>>,
    base_type: BaseCustomType,
  ) -> Result<Type<'ast>>
  {
    let is_group = base_type == BaseCustomType::CollectableGroup;
    // Collectables support 'has upgrades', 'has redemptions', and 'property'.
    // The first statement can optionally be 'has amount'.
    // Groups also support 'has collectable' and 'has collectable group'.
    // All their properties, redemptions, and upgrades are inherited
    // by the children in those lists.
    let mut auto_grouping = AutoGrouping::Inherit;
    if self.token == Keyword::Has && self.peek()? == Keyword::Amount {
      self.advance()?;
      self.advance()?;
      auto_grouping = AutoGrouping::ByAmount;
      self.consume(TokenKind::Semicolon)?;
    }
    if is_group {
      let group = self.parse_collectable_group(label)?;
      Ok(group.into())
    } else {
      let collectable = self.parse_collectable(label)?;
      Ok(collectable.into())
    }
  }

  fn parse_collectable_group(&mut self, label: TokenValue<Arc<str>>)
    -> Result<CollectableGroup<'ast>>
  {
    let mut group = CollectableGroup::new(label, self.ast.clone());
    //self.all([
    //  |&mut this, &mut grp| {
    //    let list = this.parse_has_collectable()?;
    //  }
    //], group, false);
    self.consume(Keyword::Has)?;
    optional(self.parse_has_collectable(&mut group))?;
    Ok(group)
  }

  fn parse_collectable(&mut self, label: TokenValue<Arc<str>>)
    -> Result<Collectable<'ast>>
  {
    let mut collectable = Collectable::new(label);
    loop {
      if self.opt_consume(Keyword::Has)? {
        if self.opt_consume(Keyword::Redemptions)? {
          // TODO: write these. Note: no duplicates.
          self.parse_redemptions(&mut collectable)?;
        } else if self.opt_consume(Keyword::Upgrades)? {
          self.parse_upgrades(&mut collectable)?;
        } else {
          break;
        }
      } else if self.opt_consume(Keyword::Property)? {
        //collectable.add_property(self.parse_property()?)?;
      } else {
        break;
      }
    }
    Ok(collectable)
  }

  fn parse_inline_collectable(
    &mut self,
    group: &mut CollectableGroup<'ast>,
  ) -> Result<()>
  {
    if self.token == TokenMatch::Identifier {
      group.insert_ref_mut(ItemRefMut::new(self.string_token_value()))?;
      self.advance()?;
      //self.all(&[Self::parse_upgrades, Self::parse_redemptions], group, false)?;
    }
    Ok(())
  }

  fn parse_inline_collectable_group(
    &mut self,
    group: &mut CollectableGroup<'ast>,
  ) -> Result<()>
  {
    self.consume(TokenMatch::Identifier)?;
    Ok(())
  }

  fn parse_has_collectable(
    &mut self,
    group: &mut CollectableGroup<'ast>,
  ) -> Result<()>
  {
    self.consume(Keyword::Collectable)?;
    let parser = if self.opt_consume(Keyword::Group)? {
      Self::parse_inline_collectable_group
    } else {
      Self::parse_inline_collectable
    };
    // TODO: Also allow a single ident?
    self.parse_bracketed_list(parser, group)?;
    self.consume(TokenKind::Semicolon)?;
    Ok(())
  }

  fn parse_upgrades(
    &mut self,
    container: &mut Collectable<'ast>,
  ) -> Result<()>
  {
    self.consume(Keyword::Upgrades)?;
    Ok(())
  }

  fn parse_redemptions(
    &mut self,
    container: &mut Collectable<'ast>,
  ) -> Result<()>
  {
    self.consume(Keyword::Redemptions)?;
    Ok(())
  }

  // ===== Event =====

  fn parse_event(&mut self, label: TokenValue<Arc<str>>) -> Result<Type<'ast>> {
    self.consume(Keyword::Event)?;
    unimplemented!()
  }

  // ===== Variables =====
/*
  fn parse_property(&mut self) -> Result<Property> {
    self.consume(Keyword::Property)?;
    //let ty = self.parse_full_type_name()?;
  }

  fn parse_full_type_name(&mut self) -> Result<TypeRef> {
    let token = self.take(TokenKind::Keyword)?;
    let kw = extract!(self, Keyword)?;
    Ok(match kw {
      Keyword::Switch => TypeRef::Primitive(PrimitiveType::Switch),
      Keyword::Text => TypeRef::Primitive(PrimitiveType::Text),
      Keyword::Localized => {
        self.opt_consume(Keyword::Text)?;
        TypeRef::Primitive(PrimitiveType::LocalizedText)
      }
      Keyword::Integer => TypeRef::Primitive(PrimitiveType::Integer),
      Keyword::Decimal => TypeRef::Primitive(PrimitiveType::Decimal),
      Keyword::Datetime => TypeRef::Primitive(PrimitiveType::DateTime),
      Keyword::Timespan => TypeRef::Primitive(PrimitiveType::TimeSpan),
      Keyword::Object => {
        if self.token == TokenMatch::Identifier {
          let tv = self.string_token_value();
          self.advance()?;
          TypeRef::Custom(ItemRef::new(tv))
        } else {
          TypeRef::Primitive(PrimitiveType::Object)
        }
      }
      Keyword::Array => ,
      Keyword::Remote => (),
      Keyword::User => (),
      Keyword::Collectable => (),
      Keyword::Event => (),
      Keyword::Function => (),
    })
  }
*/

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

  fn e_expected<T: Into<String>, O>(&self, t: T) -> Result<O> {
    Err(ErrorKind::Expected(t.into(), self.string_token_value()).into())
  }

  fn e_unexpected<O>(&self) -> Result<O> {
    Err(ErrorKind::Unexpected(self.string_token_value()).into())
  }

  fn e_syntax<T: Into<String>, O>(&self, msg: T) -> Result<O> {
    Err(ErrorKind::Syntax(msg.into(), self.token.span.clone()).into())
  }

  /// Move to the next token, returning the current if it matches.
  fn take<T: PartialEq<Token<'p>> + AsRef<str>>(&mut self, t: T) -> Result<Token<'p>> {
    if &t == &self.token {
      let token = self.token.clone();
      self.advance()?;
      Ok(token)
    } else {
      self.e_expected(t.as_ref())
    }
  }

  /// Return the current token if it matches, otherwise error. Don't advance.
  fn expect<T: PartialEq<Token<'p>> + AsRef<str>>(&mut self, t: T) -> Result<Token<'p>> {
    if &t == &self.token {
      Ok(self.token.clone())
    } else {
      self.e_expected(t.as_ref())
    }
  }

  /// Move to the next token if the current matches, otherwise error.
  fn consume<T: PartialEq<Token<'p>> + AsRef<str>>(&mut self, t: T) -> Result<()> {
    if &t == &self.token {
      self.advance()?;
      Ok(())
    } else {
      self.e_expected(t.as_ref())
    }
  }

  /// Move to the next token if the current matches, otherwise false.
  fn opt_consume<T: PartialEq<Token<'p>>>(&mut self, t: T) -> Result<bool> {
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
