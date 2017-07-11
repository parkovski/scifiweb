use super::Target;

pub struct Message<'a> {
  text: String,
  target: Target<'a>,
}