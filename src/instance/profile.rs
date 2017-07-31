pub enum ProfileId {
  Facebook(u64),
  GameCenter(String),
  GooglePlay(String),
  Name(String),
  InternalId(u64),
}

pub struct Profile {
  //rules: &'a rules::Profile<'a>,
  id: ProfileId,
}
