use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::env;
use std::convert::{TryFrom, TryInto};
use std::result::Result as StdResult;
use nom::IResult;
use fxhash::FxHashSet;
use util::split_vec::SplitVec;
use util::graph_cell::*;
use ast::*;
use ast::ty::*;
use ast::var::*;
use ast::expr::*;
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

impl<'a> TryFrom<TokenKind<'a>> for PrefixOperator {
  type Error = ();
  fn try_from(value: TokenKind<'a>) -> StdResult<Self, ()> {
    match value {
      TokenKind::LParen => Ok(PrefixOperator::Parens),
      TokenKind::Minus => Ok(PrefixOperator::Neg),
      TokenKind::Exclamation => Ok(PrefixOperator::Not),
      TokenKind::Dot => Ok(PrefixOperator::Dot),
      _ => Err(()),
    }
  }
}

impl<'a> TryFrom<TokenKind<'a>> for BinaryOperator {
  type Error = ();
  fn try_from(value: TokenKind<'a>) -> StdResult<Self, ()> {
    Ok(match value {
      TokenKind::Dot => BinaryOperator::Dot,
      TokenKind::Multiply => BinaryOperator::Mul,
      TokenKind::Divide => BinaryOperator::Div,
      TokenKind::PercentSign => BinaryOperator::Mod,
      TokenKind::Caret => BinaryOperator::Pow,
      TokenKind::Plus => BinaryOperator::Add,
      TokenKind::Minus => BinaryOperator::Sub,
      TokenKind::Equal => BinaryOperator::Eq,
      TokenKind::NotEqual => BinaryOperator::Ne,
      TokenKind::Less => BinaryOperator::Lt,
      TokenKind::LessEqual => BinaryOperator::Le,
      TokenKind::Greater => BinaryOperator::Gt,
      TokenKind::GreaterEqual => BinaryOperator::Ge,
      TokenKind::Keyword(Keyword::And) => BinaryOperator::And,
      TokenKind::Keyword(Keyword::Or) => BinaryOperator::Or,
      _ => return Err(()),
    })
  }
}

impl<'a> TryFrom<TokenKind<'a>> for PostfixListOperator {
  type Error = ();
  fn try_from(value: TokenKind<'a>) -> StdResult<Self, ()> {
    Ok(match value {
      TokenKind::LParen => PostfixListOperator::Call,
      TokenKind::LSquareBracket => PostfixListOperator::Idx,
      _ => return Err(())
    })
  }
}

pub struct Parser<'p, 'ast: 'p> {
  filename: Arc<PathBuf>,
  token: Token<'p>,
  included_paths: &'p mut FxHashSet<Arc<PathBuf>>,
  inp: &'p [u8],
  ast: GraphRefMut<'ast, Ast<'ast>>,
}

// TODO: Remove when this is finished.
#[allow(dead_code)]
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
    filename: &Path,
    program: &'p str,
    ast: GraphRefMut<'ast, Ast<'ast>>,
  ) -> Result<()>
  {
    let mut includes = Default::default();
    let mut parser = Self::new(
      Arc::new(filename.into()),
      &mut includes,
      program.as_bytes(),
      ast,
    );
    parser.parse_program()?;
    ast.awake().typecheck()?;
    Ok(())
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
      } else {
        let base_type = self.parse_base_custom_type()?;
        self.expect(TokenMatch::Identifier)?;
        let label = self.string_token_value();
        self.advance()?;
        // An item (type) definition
        if self.opt_consume(TokenKind::Semicolon)? {
          // Empty item
          base_type.insert_empty_type(self.ast, label)?;
        } else {
          self.consume(TokenKind::Colon)?;
          // FIXME: These give out pointers to their scope, so they must
          // be created in place and not moved!
          match base_type {
            | BaseCustomType::EarlyRef => unreachable!(),
            | BaseCustomType::Collectable
              => self.parse_collectable(label),
            | BaseCustomType::CollectableGroup
              => self.parse_collectable_group(label),
            | BaseCustomType::User
              => self.parse_user(label),
            | BaseCustomType::UserGroup
              => self.parse_user_group(label),
            | BaseCustomType::Event
              => self.parse_event(label),
            | BaseCustomType::RemoteEvent
              => self.parse_remote_event(label),
            | BaseCustomType::Function
              => self.parse_function(label),
            | BaseCustomType::RemoteFunction
              => self.parse_remote_function(label),
            | BaseCustomType::Object
              => self.parse_object_type(label),
            | BaseCustomType::Array
              => return self.e_syntax("custom array types are defined inline"),
          }?;
          self.parse_end()?;
        }
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
      let item_ref = ItemRef::new(self.string_token_value(), self.ast.asleep_ref());
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
            self.expect(TokenMatch::Integer)?;
            let tv = self.int_token_value().unwrap();
            self.advance()?;
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
          let type_name = ty.map(|t| t.name().clone());
          let array = Ast::get_array(self.ast, ArrayName::new(length, type_name));
          return Ok(
            ItemRef::with_item(array.awake().name().clone(), array)
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

  fn parse_user(&mut self, label: TokenValue<Arc<str>>) -> Result<()> {
    User::new(label, self.ast)?;
    Ok(())
  }

  fn parse_user_group(&mut self, label: TokenValue<Arc<str>>) -> Result<()> {
    UserGroup::new(label, self.ast)?;
    Ok(())
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
    -> Result<()>
  {
    let _group = CollectableGroup::new(label, self.ast)?;
    let mut group = _group.awake_mut();
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
        let scope = group.scope_mut();
        let prop = self.parse_property(scope)?;
        scope.awake_mut().insert(prop)?;
        self.consume(TokenKind::Semicolon)?;
      } else if self.token == Keyword::Has {
        if !Self::all_done(&vec) {
          self.advance()?;
          self.all_next(&mut vec, &mut *group)?;
          self.consume(TokenKind::Semicolon)?;
        } else {
          return self.e_syntax("only one of each has * block allowed");
        }
      } else {
        return Ok(());
      }
    }
  }

  fn parse_collectable(&mut self, label: TokenValue<Arc<str>>)
    -> Result<()>
  {
    let _collectable = Collectable::new(label, self.ast)?;
    let mut collectable = _collectable.awake_mut();
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
        let scope = collectable.scope_mut();
        let prop = self.parse_property(scope)?;
        scope.awake_mut().insert(prop)?;
        self.consume(TokenKind::Semicolon)?;
      } else if self.token == Keyword::Has {
        if !Self::all_done(&vec) {
          self.advance()?;
          self.all_next(&mut vec, &mut *collectable)?;
          self.consume(TokenKind::Semicolon)?;
        } else {
          return self.e_syntax("only one of each `has *` block allowed");
        }
      } else {
        return Ok(());
      }
    }
  }

  fn parse_inline_collectable(&mut self) -> Result<EarlyRefType<'ast>> {
    self.expect(TokenMatch::Identifier)?;
    let ty = EarlyRefType::new(
      self.string_token_value(),
      BaseCustomType::Collectable
    );
    self.advance()?;
    Ok(ty)
  }

  fn parse_inline_collectable_group(&mut self) -> Result<EarlyRefType<'ast>> {
    self.expect(TokenMatch::Identifier)?;
    let ty = EarlyRefType::new(
      self.string_token_value(),
      BaseCustomType::CollectableGroup
    );
    self.advance()?;
    Ok(ty)
  }

  fn parse_has_collectable_or_group(
    &mut self,
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

    let add_item = move |this: &mut Self| -> Result<()> {
      let item = inline_item(this)?;
      Ast::insert_type(this.ast, item)?;
      Ok(())
    };

    if self.token == TokenKind::LSquareBracket {
      self.parse_delimited_list_unit(
        TokenKind::LSquareBracket,
        TokenKind::Comma,
        TokenKind::RSquareBracket,
        add_item,
      )
    } else {
      add_item(self)
    }
  }

  fn parse_has_collectable(&mut self, _: &mut CollectableGroup<'ast>)
    -> Result<()>
  {
    self.parse_has_collectable_or_group(false)
  }

  fn parse_has_collectable_group(&mut self, _: &mut CollectableGroup<'ast>)
    -> Result<()>
  {
    self.parse_has_collectable_or_group(true)
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

  fn parse_event(&mut self, label: TokenValue<Arc<str>>) -> Result<()> {
    Event::new(label, self.ast)?;
    Ok(())
  }

  fn parse_remote_event(&mut self, label: TokenValue<Arc<str>>) -> Result<()> {
    RemoteEvent::new(label, self.ast)?;
    Ok(())
  }

  // <>Function

  fn parse_function(&mut self, label: TokenValue<Arc<str>>) -> Result<()> {
    Function::new(label, self.ast)?;
    Ok(())
  }

  fn parse_remote_function(&mut self, label: TokenValue<Arc<str>>) -> Result<()> {
    RemoteFunction::new(label, self.ast)?;
    Ok(())
  }

  // <>Object

  fn parse_object_type(&mut self, label: TokenValue<Arc<str>>) -> Result<()> {
    Object::new(label, self.ast)?;
    Ok(())
  }

  // <>Variable

  /// property <name> <type>
  fn parse_property(&mut self, scope: GraphRefMut<'ast, Scope<'ast>>)
    -> Result<Variable<'ast>>
  {
    self.expect(TokenMatch::Identifier)?;
    let name = self.string_token_value();
    self.advance()?;
    let ty = self.parse_type()?;
    let mut var = Variable::new(name, ty);
    if self.token == TokenKind::Equal {
      self.advance()?;
      var.set_initial(self.parse_expression(scope)?);
    }
    Ok(var)
  }

  // <>Expression

  fn parse_expression(&mut self, scope: GraphRefMut<'ast, Scope<'ast>>)
    -> Result<BoxExpression<'ast>>
  {
    self.parse_precedence_expr(0, scope)
  }

  fn parse_precedence_expr(
    &mut self,
    precedence: u8,
    scope: GraphRefMut<'ast, Scope<'ast>>
  )
    -> Result<BoxExpression<'ast>>
  {
    let mut expr: BoxExpression<'ast>;

    if let Some(prefix) = self.prefix_token_value() {
      let is_paren = *prefix.value() == PrefixOperator::Parens;
      self.advance()?;
      let precedence = prefix.value().precedence();
      expr = box PrefixExpr::new(
        prefix,
        self.parse_precedence_expr(precedence, scope)?
      );
      if is_paren {
        self.consume(TokenKind::RParen)?;
      }
    } else {
      expr = self.parse_primary_expr(scope)?;
    }

    loop {
      if let Some(postfix) = self.postfix_token_value() {
        if PostfixListOperator::PRECEDENCE < precedence {
          break;
        }
        let close = match *postfix.value() {
          PostfixListOperator::Call => TokenKind::RParen,
          PostfixListOperator::Idx => TokenKind::RSquareBracket,
        };
        self.advance()?;
        let list = self.parse_list(
          TokenKind::Comma,
          |this| this.parse_expression(scope),
          Vec::new(),
          Vec::push,
        )?;
        expr = box PostfixListExpr::new(postfix, expr, list);
        self.consume(close)?;
      } else if let Some(binary) = self.binary_token_value() {
        let binary_precedence = binary.value().precedence();
        if binary_precedence < precedence {
          break;
        }

        let next_precedence = if binary.value().right_recursive() {
          binary_precedence
        } else {
          binary_precedence + 1
        };
        self.advance()?;
        expr = box BinaryExpr::new(
          binary,
          expr,
          self.parse_precedence_expr(next_precedence, scope)?
        );
      } else {
        break;
      }
    }
    Ok(expr)
  }

  /// primary = ident | amount | literal
  fn parse_primary_expr(&mut self, scope: GraphRefMut<'ast, Scope<'ast>>)
    -> Result<BoxExpression<'ast>>
  {
    if self.token == TokenMatch::Identifier {
      let tv = self.string_token_value();
      self.advance()?;
      Ok(box ExprVar::new(tv, scope.asleep_ref().into()))
    } else if self.token == Keyword::Amount {
      let tv = self.string_token_value();
      self.advance()?;
      Ok(box ExprVar::new(tv, scope.asleep_ref().into()))
    } else if self.token == TokenMatch::Decimal || self.token == TokenMatch::Percentage {
      let tv = self.float_token_value().unwrap();
      self.advance()?;
      Ok(box ExprLiteral::new(
        Literal::Decimal(tv),
        self.ast.awake().primitive().decimal(),
      ))
    } else if self.token == TokenMatch::Integer {
      let tv = self.int_token_value().unwrap();
      self.advance()?;
      Ok(match self.parse_time_span(&tv) {
        Some(ts) => box ExprLiteral::new(
          Literal::TimeSpan(ts),
          self.ast.awake().primitive().time_span()
        ),
        None => box ExprLiteral::new(
          Literal::Integer(tv),
          self.ast.awake().primitive().integer()
        ),
      })
    } else if self.token == TokenMatch::String {
      let tv = self.string_token_value();
      self.advance()?;
      Ok(box ExprLiteral::new(
        Literal::Text(tv),
        self.ast.awake().primitive().text()
      ))
    } else if self.token == Keyword::Localized {
      let loc_span = self.token.span.clone();
      self.advance()?;
      let tok = self.take(TokenMatch::String)?;
      let s = extract!(self, String in tok).unwrap();
      let s = self.ast.awake().shared_string(s);
      let tv = TokenValue::new(s, loc_span.from_to(&tok.span));
      Ok(box ExprLiteral::new(
        Literal::LocalizedText(tv),
        self.ast.awake().primitive().localized_text()
      ))
    } else if self.token == Keyword::No {
      let tv = TokenValue::new(false, self.token.span.clone());
      self.advance()?;
      Ok(box ExprLiteral::new(
        Literal::Option(tv),
        self.ast.awake().primitive().option()
      ))
    } else if self.token == Keyword::Yes {
      let tv = TokenValue::new(true, self.token.span.clone());
      self.advance()?;
      Ok(box ExprLiteral::new(
        Literal::Option(tv),
        self.ast.awake().primitive().option()
      ))
    } else {
      self.e_unexpected()
    }
  }

  fn parse_time_span(&mut self, _: &TokenValue<i64>) -> Option<Vec<TimeSpanPart>> {
    None
  }

/*
  // TODO:
  /// localized <constant expression>
  /// converts all string literals in the expression to localized strings.
  fn parse_localized(&mut self) -> Option<BoxExpression<'ast>> {

  }
*/

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
    folder: F,
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
        folder,
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

  fn int_token_value(&self) -> Option<TokenValue<i64>> {
    match self.token.kind {
      TokenKind::Integer(i) => Some(TokenValue::new(i, self.token.span.clone())),
      _ => None,
    }
  }

  fn float_token_value(&self) -> Option<TokenValue<f64>> {
    match self.token.kind {
      TokenKind::Decimal(d) => Some(TokenValue::new(d, self.token.span.clone())),
      TokenKind::Percentage(p) => Some(TokenValue::new(p / 100.0, self.token.span.clone())),
      TokenKind::Integer(i) => Some(TokenValue::new(i as f64, self.token.span.clone())),
      _ => None,
    }
  }

  fn prefix_token_value(&self) -> Option<TokenValue<PrefixOperator>> {
    let oper: StdResult<PrefixOperator, ()> = self.token.kind.try_into();
    match oper {
      Ok(oper) => Some(TokenValue::new(oper, self.token.span.clone())),
      Err(()) => None,
    }
  }

  fn binary_token_value(&self) -> Option<TokenValue<BinaryOperator>> {
    let oper: StdResult<BinaryOperator, ()> = self.token.kind.try_into();
    match oper {
      Ok(oper) => Some(TokenValue::new(oper, self.token.span.clone())),
      Err(()) => None,
    }
  }

  fn postfix_token_value(&self) -> Option<TokenValue<PostfixListOperator>> {
    let oper: StdResult<PostfixListOperator, ()> = self.token.kind.try_into();
    match oper {
      Ok(oper) => Some(TokenValue::new(oper, self.token.span.clone())),
      Err(()) => None,
    }
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
