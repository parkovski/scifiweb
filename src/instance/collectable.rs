use rules;

pub struct Collectable<'a> {
  rules: &'a rules::Collectable<'a>,
  id: u64,
  name: String,
  level: i32,
  amount: i32,
}
