import matchRouter from './match';
import playerRouter from './player';

import express from 'express';
import { Pool } from 'pg';

const app = express();
const pool = new Pool({
  user: 'scifi',
  password: 'scifi',
  host: 'localhost',
  database: 'scifi',
  max: 10,
  idleTimeoutMillis: 5000,
});

app.get('/', (req, res) => {
  res.end('I bet you\'ll find this site super useful.');
});

app.use('/match', matchRouter(pool));
app.use('/player', playerRouter(pool));

const port = process.env.PORT || 3000;
app.listen(port, () => {
  console.log(`Listening on port ${port}.`);
});
