export default function authorize(req, res, next) {
  if (req.query.auth === 'secret') {
    next();
  } else {
    res.writeHead(403);
    res.end('Not authorized');
  }
};