use rules::collectable::Collectable as CollectableRules;

pub struct Collectable<'a> {
  rules: &'a CollectableRules<'a>,
  id: u64,
  name: String,
  level: i32,
  amount: i32,
}
