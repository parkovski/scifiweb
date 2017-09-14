use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::env;
use std::fmt::{Debug, Display};
use std::convert::TryInto;
use nom::IResult;
use fxhash::FxHashSet;
use util::split_vec::SplitVec;
use util::graph_cell::*;
use ast::*;
use ast::ty::*;
use ast::var::*;
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

trait SyntaxConsumer<'p, 'ast: 'p>: Copy {
  fn consume(&self, p: &mut Parser<'p, 'ast>) -> Result<()>;
  fn opt_consume(&self, p: &mut Parser<'p, 'ast>) -> Result<bool> {
    Ok(optional(self.consume(p))?.is_some())
  }
}

impl<'f, 'p: 'f, 'ast: 'p> SyntaxConsumer<'p, 'ast>
for &'f fn(&mut Parser<'p, 'ast>) -> Result<()>
{
  fn consume(&self, p: &mut Parser<'p, 'ast>) -> Result<()> {
    self(p)
  }
}

impl<'p, 'ast, T> SyntaxConsumer<'p, 'ast> for T
where
  'ast: 'p,
  T: PartialEq<Token<'p>> + AsRef<str> + Copy,
{
  fn consume(&self, p: &mut Parser<'p, 'ast>) -> Result<()> {
    if self == p.current_token() {
      p.advance()
    } else {
      p.e_expected(self.as_ref())
    }
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

  // <>Program

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
          base_type.insert_empty_type(self.ast, label)?;
        } else {
          self.consume(TokenKind::Colon)?;
          match base_type {
            | BaseCustomType::Collectable => {
              Ast::insert_type(self.ast, self.parse_collectable(label)?)?;
            }
            | BaseCustomType::CollectableGroup => {
              Ast::insert_type(self.ast, self.parse_collectable_group(label)?)?;
            }
            | BaseCustomType::User => {
              Ast::insert_type(self.ast, self.parse_user(label)?)?;
            }
            | BaseCustomType::UserGroup => {
              Ast::insert_type(self.ast, self.parse_user_group(label)?)?;
            }
            | BaseCustomType::Event => {
              Ast::insert_type(self.ast, self.parse_event(label)?)?;
            }
            | BaseCustomType::RemoteEvent => {
              Ast::insert_type(self.ast, self.parse_remote_event(label)?)?;
            }
            | BaseCustomType::Function => {
              Ast::insert_type(self.ast, self.parse_function(label)?)?;
            }
            | BaseCustomType::RemoteFunction => {
              Ast::insert_type(self.ast, self.parse_remote_function(label)?)?;
            }
            | BaseCustomType::Object => {
              Ast::insert_type(self.ast, self.parse_object_type(label)?)?;
            }
            | BaseCustomType::Array
              => return self.e_syntax("custom array types are defined inline"),
          }
          self.parse_end()?;
        }
      } else {
        return self.e_unexpected();
      }
    }
  }

  // <>Types

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

  fn parse_type(&mut self) -> Result<ItemRef<'ast, Type<'ast>>> {
    if self.token == TokenMatch::Identifier {
      let item_ref = ItemRef::new(self.string_token_value());
      self.advance()?;
      Ok(item_ref)
    } else if self.token == TokenMatch::Keyword {
      let keyword = extract!(self, Keyword).unwrap();
      let primitive_type = match keyword {
        Keyword::Option => PrimitiveType::Option,
        Keyword::Text => PrimitiveType::Text,
        Keyword::Localized => {
          self.advance()?;
          self.expect(Keyword::Text)?;
          PrimitiveType::LocalizedText
        }
        Keyword::Integer => PrimitiveType::Integer,
        Keyword::Decimal => PrimitiveType::Decimal,
        Keyword::Datetime => PrimitiveType::DateTime,
        Keyword::Timespan => PrimitiveType::TimeSpan,
        Keyword::Object => PrimitiveType::Object,
        Keyword::Array => {
          // "array" is a primitive type,
          // but there are also sized and typed arrays:
          // array (x length) (of type)
          let mut next = self.take_next()?;
          let mut length = None;
          let mut ty = None;
          if next == Keyword::X {
            // TODO: Constant expression
            self.advance()?;
            let len_i64 = self.take(TokenMatch::Integer)?;
            let tv = self.token_value(extract!(self, Integer in len_i64)?);
            let len_u32: u32 = (*tv.value()).try_into()
              .or_else(|_| -> Result<u32> {
                Err(ErrorKind::IntegerOutOfRange(
                  tv, "array length must be 32-bit unsigned"
                ).into())
              })?;
            length = Some(len_u32);
            next = self.token.clone();
          }
          if next == Keyword::Of {
            self.advance()?;
            ty = Some(self.parse_type()?);
          }
          // TODO: Return the custom type for the array.
          let type_name = ty.map(|t| t.source_name().clone());
          let array = Ast::get_array(self.ast, ArrayName::new(length, type_name));
          return Ok(
            ItemRef::with_item(array.awake().source_name().clone(), array)
          );
        }
        _ => return self.e_expected("type name"),
      };
      let tv = self.string_token_value();
      let gr = <Ast as Owner<Type>>::find(&self.ast.awake(), primitive_type.as_str()).unwrap();
      self.advance()?;
      Ok(ItemRef::with_item(tv.clone(), gr))
    } else {
      self.e_expected("type name")
    }
  }

  // <>Include

  fn parse_include(&mut self) -> Result<()> {
    self.consume(Keyword::Include)?;
    let path_token = self.take(TokenMatch::String)?;
    self.consume(TokenKind::Semicolon)?;
    let path = extract!(self, String in path_token)?;
    self.include(path)?;
    Ok(())
  }

  // <>User

  fn parse_user(&mut self, label: TokenValue<Arc<str>>) -> Result<User<'ast>> {
    unimplemented!()
  }

  fn parse_user_group(&mut self, label: TokenValue<Arc<str>>)
    -> Result<UserGroup<'ast>>
  {
    unimplemented!()
  }

  // <>Collectable

  fn parse_auto_grouping(&mut self) -> Result<AutoGrouping> {
    if self.token == Keyword::Has && self.peek()? == Keyword::Amount {
      self.advance_x(2)?;
      self.consume(TokenKind::Semicolon)?;
      Ok(AutoGrouping::ByAmount)
    } else {
      Ok(AutoGrouping::Inherit)
    }
  }

  fn parse_collectable_group(&mut self, label: TokenValue<Arc<str>>)
    -> Result<CollectableGroup<'ast>>
  {
    let mut group = CollectableGroup::new(label);
    group.set_auto_grouping(self.parse_auto_grouping()?);
    let mut vec = Self::all_init(&[
      Self::parse_has_collectable,
      Self::parse_has_collectable_group,
      |this: &mut Self, ref mut grp| -> Result<()> {
        Ok(grp.insert_redemptions(this.parse_redemptions()?))
      },
      |this: &mut Self, ref mut grp| -> Result<()> {
        Ok(grp.insert_upgrades(this.parse_upgrades()?))
      }
    ]);
    loop {
      if self.opt_consume(Keyword::Property)? {
        group.insert_property(self.parse_property()?)?;
        self.consume(TokenKind::Semicolon)?;
      } else if self.token == Keyword::Has {
        if !Self::all_done(&vec) {
          self.advance()?;
          self.all_next(&mut vec, &mut group)?;
          self.consume(TokenKind::Semicolon)?;
        } else {
          return self.e_syntax("only one of each has * block allowed");
        }
      } else {
        return Ok(group);
      }
    }
  }

  fn parse_collectable(&mut self, label: TokenValue<Arc<str>>)
    -> Result<Collectable<'ast>>
  {
    let mut collectable = Collectable::new(label);
    collectable.set_auto_grouping(self.parse_auto_grouping()?);
    let mut vec = Self::all_init(&[
      |this: &mut Self, coll: &mut Collectable<'ast>| -> Result<()> {
        Ok(coll.insert_redemptions(this.parse_redemptions()?))
      },
      |this: &mut Self, coll: &mut Collectable<'ast>| -> Result<()> {
        Ok(coll.insert_upgrades(this.parse_upgrades()?))
      }
    ]);
    loop {
      if self.opt_consume(Keyword::Property)? {
        collectable.insert_property(self.parse_property()?)?;
        self.consume(TokenKind::Semicolon)?;
      } else if self.token == Keyword::Has {
        if !Self::all_done(&vec) {
          self.advance()?;
          self.all_next(&mut vec, &mut collectable)?;
          self.consume(TokenKind::Semicolon)?;
        } else {
          return self.e_syntax("only one of each has * block allowed");
        }
      } else {
        return Ok(collectable);
      }
    }
  }

  fn parse_inline_collectable(
    &mut self,
    group: &mut CollectableGroup<'ast>,
  ) -> Result<()>
  {
    self.expect(TokenMatch::Identifier)?;
    group.insert_collectable_ref(ItemRefMut::new(self.string_token_value()))?;
    self.advance()?;
    //self.all(&[Self::parse_upgrades, Self::parse_redemptions], group, false)?;
    Ok(())
  }

  fn parse_inline_collectable_group(
    &mut self,
    group: &mut CollectableGroup<'ast>,
  ) -> Result<()>
  {
    self.expect(TokenMatch::Identifier)?;
    group.insert_group_ref(ItemRefMut::new(self.string_token_value()))?;
    self.advance()?;
    Ok(())
  }

  fn parse_has_collectable_or_group(
    &mut self,
    group: &mut CollectableGroup<'ast>,
    is_inline_group: bool,
  ) -> Result<()>
  {
    let peek = self.peek()?;
    if self.token != Keyword::Collectable {
      return self.e_expected("collectable");
    } else if is_inline_group && peek != Keyword::Group {
      return self.e_expected("group");
    } else if !is_inline_group && peek == Keyword::Group {
      return self.e_unexpected();
    }
    let inline_item = if is_inline_group {
      self.advance_x(2)?;
      Self::parse_inline_collectable_group
    } else {
      self.advance()?;
      Self::parse_inline_collectable
    };

    if self.token == TokenKind::LSquareBracket {
      self.parse_delimited_list_unit(
        TokenKind::LSquareBracket,
        TokenKind::Comma,
        TokenKind::RSquareBracket,
        move |this| inline_item(this, group),
      )
    } else {
      inline_item(self, group)
    }
  }

  fn parse_has_collectable(
    &mut self,
    group: &mut CollectableGroup<'ast>,
  ) -> Result<()>
  {
    self.parse_has_collectable_or_group(group, false)
  }

  fn parse_has_collectable_group(
    &mut self,
    group: &mut CollectableGroup<'ast>,
  ) -> Result<()>
  {
    self.parse_has_collectable_or_group(group, true)
  }

  fn parse_upgrades(
    &mut self,
  ) -> Result<Vec<Upgrade>>
  {
    self.consume(Keyword::Upgrades)?;
    Ok(Vec::new())
  }

  fn parse_redemptions(
    &mut self,
  ) -> Result<Vec<Redemption>>
  {
    self.consume(Keyword::Redemptions)?;
    Ok(Vec::new())
  }

  // <>Event

  fn parse_event(&mut self, label: TokenValue<Arc<str>>) -> Result<Event<'ast>> {
    unimplemented!()
  }

  fn parse_remote_event(&mut self, label: TokenValue<Arc<str>>)
    -> Result<RemoteEvent<'ast>>
  {
    unimplemented!()
  }

  // <>Function

  fn parse_function(&mut self, label: TokenValue<Arc<str>>) -> Result<Function<'ast>> {
    unimplemented!()
  }

  fn parse_remote_function(&mut self, label: TokenValue<Arc<str>>)
    -> Result<RemoteFunction<'ast>>
  {
    unimplemented!()
  }

  // <>Object

  fn parse_object_type(&mut self, label: TokenValue<Arc<str>>) -> Result<Object<'ast>> {
    unimplemented!()
  }

  // <>Variables

  /// property <name> <type>
  fn parse_property(&mut self) -> Result<Variable<'ast>> {
    self.expect(TokenMatch::Identifier)?;
    let name = self.string_token_value();
    self.advance()?;
    let ty = self.parse_type()?;
    Ok(Variable::new(name, ty))
  }

  // <>General

  fn parse_end(&mut self) -> Result<()> {
    self.consume(Keyword::End)?;
    self.consume(TokenKind::Semicolon)
  }

  // <>Helpers

  fn parse_list<C, P, I, A, F>(
    &mut self,
    separator: C,
    mut parser: P,
    mut accumulator: A,
    mut fold_item: F,
  ) -> Result<A>
  where
    C: SyntaxConsumer<'p, 'ast>,
    P: FnMut(&mut Self) -> Result<I>,
    F: FnMut(&mut A, I),
  {
    loop {
      let item = if let Some(item) = optional(parser(self))? {
        item
      } else {
        break
      };
      fold_item(&mut accumulator, item);
      if !separator.opt_consume(self)? {
        break;
      }
    }
    Ok(accumulator)
  }

  fn parse_delimited<Co, Cc, F, R>(
    &mut self,
    open: Co,
    close: Cc,
    inner: F,
  ) -> Result<R>
  where
    Co: SyntaxConsumer<'p, 'ast>,
    Cc: SyntaxConsumer<'p, 'ast>,
    F: FnOnce(&mut Self) -> Result<R>,
  {
    open.consume(self)?;
    let result = inner(self)?;
    close.consume(self)?;
    Ok(result)
  }

  fn parse_delimited_list<Co, Cs, Cc, P, I, A, F>(
    &mut self,
    open: Co,
    separator: Cs,
    close: Cc,
    parser: P,
    accumulator: A,
    fold_item: F,
  ) -> Result<A>
  where
    Co: SyntaxConsumer<'p, 'ast>,
    Cs: SyntaxConsumer<'p, 'ast>,
    Cc: SyntaxConsumer<'p, 'ast>,
    P: FnMut(&mut Self) -> Result<I>,
    F: FnMut(&mut A, I),
  {
    self.parse_delimited(
      open,
      close,
      |this| this.parse_list(
        separator,
        parser,
        accumulator,
        fold_item,
      ),
    )
  }

  fn parse_delimited_list_unit<Co, Cs, Cc, P>(
    &mut self,
    open: Co,
    separator: Cs,
    close: Cc,
    parser: P,
  ) -> Result<()>
  where
    Co: SyntaxConsumer<'p, 'ast>,
    Cs: SyntaxConsumer<'p, 'ast>,
    Cc: SyntaxConsumer<'p, 'ast>,
    P: FnMut(&mut Self) -> Result<()>,
  {
    self.parse_delimited(
      open,
      close,
      move |this| this.parse_list(
        separator,
        parser,
        (),
        |&mut (), ()| {},
      )
    )
  }

  // <>Errors

  fn e_expected<T: Into<String>, O>(&self, t: T) -> Result<O> {
    Err(ErrorKind::Expected(t.into(), self.string_token_value()).into())
  }

  pub fn e_unexpected<O>(&self) -> Result<O> {
    Err(ErrorKind::Unexpected(self.string_token_value()).into())
  }

  fn e_syntax<T: Into<String>, O>(&self, msg: T) -> Result<O> {
    Err(ErrorKind::Syntax(msg.into(), self.token.span.clone()).into())
  }

  fn e_invalid<O>(&self, msg: &'static str) -> Result<O> {
    Err(ErrorKind::InvalidOperation(msg, self.token.span.clone()).into())
  }

  // <>Tokens

  fn string_token_value(&self) -> TokenValue<Arc<str>> {
    let kind = self.token.kind.as_str();
    let ss = self.ast.awake().shared_string(kind);
    let span = self.token.span.clone();
    TokenValue::new(ss, span)
  }

  fn token_value<I>(&self, value: I) -> TokenValue<I>
  where I: Debug + Display + Clone + PartialEq
  {
    TokenValue::new(value, self.token.span.clone())
  }

  fn lexer_iresult(&self) -> Result<(Token<'p>, &'p [u8])> {
    match lexer::next_token(self.inp, &self.token.span) {
      IResult::Done(inp, token) => Ok((token, inp)),
      IResult::Incomplete(_) => unreachable!("Lexer should not return incomplete"),
      IResult::Error(e) => Err(Error::from_nom(e, &self.token.span)),
    }
  }

  pub fn current_token(&self) -> &Token<'p> {
    &self.token
  }

  pub fn advance(&mut self) -> Result<()> {
    let (token, inp) = self.lexer_iresult()?;
    self.inp = inp;
    self.token = token;
    Ok(())
  }

  fn advance_x(&mut self, times: u8) -> Result<()> {
    for _ in 0..times {
      self.advance()?;
    }
    Ok(())
  }

  fn peek(&self) -> Result<Token<'p>> {
    let (token, _) = self.lexer_iresult()?;
    Ok(token)
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

  fn take_next(&mut self) -> Result<Token<'p>> {
    self.advance()?;
    Ok(self.token.clone())
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
  fn consume<C: SyntaxConsumer<'p, 'ast>>(&mut self, consumer: C) -> Result<()> {
    consumer.consume(self)
  }

  /// Move to the next token if the current matches, otherwise false.
  fn opt_consume<C: SyntaxConsumer<'p, 'ast>>(&mut self, consumer: C) -> Result<bool> {
    match optional(consumer.consume(self)) {
      Ok(Some(())) => Ok(true),
      Ok(None) => Ok(false),
      Err(e) => Err(e),
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

  fn all_init<P, R>(fns: &[fn(&mut Self, &mut P) -> Result<R>])
    -> SplitVec<fn(&mut Self, &mut P) -> Result<R>>
  {
    let mut sv = SplitVec::with_capacity(fns.len());
    for f in fns {
      sv.push_right(*f);
    }
    sv
  }

  fn all_next<P, R>(
    &mut self,
    fns: &mut SplitVec<fn(&mut Self, &mut P) -> Result<R>>,
    param: &mut P,
  ) -> Result<R>
  {
    let right_len = fns.right_len();
    if fns.right_len() == 0 {
      return self.e_invalid("all_next called for completed list");
    }
    let split = fns.split_index();
    for i in split..(split + right_len) {
      let f = fns[i];
      if let Some(res) = optional(f(self, param))? {
        fns.move_left(i);
        return Ok(res);
      }
    }
    self.e_unexpected()
  }

  fn all_done<F>(fns: &SplitVec<F>) -> bool {
    fns.right_len() == 0
  }
}
