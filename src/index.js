import express from 'express';
import { Pool } from 'pg';

const app = express();
const pool = new Pool({
  user: 'scifi',
  password: 'scifi',
  host: 'localhost',
  database: 'scifi',
});

function validateOnePlayerId(idStr) {
  const int = parseInt(idStr);
  if (isNaN(int) || int < 0) {
    throw 'invalid id';
  }
  return int;
}

function validatePlayerIds(playerQuery) {
  return playerQuery.map(validateOnePlayerId);
}

app.get('/', (req, res) => {
  res.end('I bet you\'ll find this site super useful.');
});

/// params: p=id (array), winner=id
app.post('/match/new', async (req, res) => {
  let players, winner;
  try {
    players = validatePlayerIds(req.query.p);
    winner = validateOnePlayerId(req.query.winner);
  } catch (e) {
    res.end('invalid id');
    return;
  }

  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    const result = await client.query(
      'INSERT INTO matches (winner) VALUES ($1) RETURNING id',
      [winner]
    );
    const match_id = result.rows[0].id;
    for (const player of players) {
      client.query(
        'INSERT INTO player_matches (player, match) VALUES ($1, $2)',
        [player, match_id]
      );
    }
    await client.query('COMMIT');
    client.release();
    res.write('created match ' + match_id);
  } catch (e) {
    client.release(true);
    res.write('match creation failed');
    console.log(e);
  } finally {
    res.end();
  }
});

/// params: name, nickname
app.post('/player/new', async (req, res) => {
  const name = req.query.name;
  const nickname = req.query.nickname;
  try {
    const result = await pool.query(
      'INSERT INTO players (name, nickname) VALUES ($1, $2) RETURNING id',
      [name, nickname]
    );
    res.end('created player ' + result.rows[0].id);
  } catch (e) {
    console.log(e);
    res.end('error creating player');
  }
});

app.get('/player/:player_id/matches', async (req, res) => {
  let player;
  try {
    player = validateOnePlayerId(req.params.player_id);
  } catch (e) {
    res.end('invalid player id');
    return;
  }

  const result = await pool.query(
    'SELECT match FROM player_matches WHERE player = $1',
    [player]
  );

  if (result.rowCount === 0) {
    res.end('no matches found');
  } else {
    let matchList = result.rows.map(r => r.match).reduce((a, b) => a + b + ',', '');
    // chop off the last ','
    matchList = matchList.substr(0, matchList.length - 1);
    res.end(matchList);
  }
});

app.get('/player/:player_id/friends', (req, res) => {
  res.end('not implemented');
});

app.get('/player/:player_id/friends/requests', (req, res) => {
  res.end('not implemented');
});

app.post('/player/:player_id/friends/requests/:friend_id/new', (req, res) => {
  res.end('not implemented');
});

app.post('/player/:player_id/friends/requests/:friend_id/approve', (req, res) => {
  res.end('not implemented');
});

app.post('/player/:player_id/friends/requests/:friend_id/deny', (req, res) => {
  res.end('not implemented');
});

app.get('/scores/all/:player_id', async (req, res) => {
  let player;
  try {
    player = validateOnePlayerId(req.params.player_id);
  } catch (e) {
    res.end('invalid player id');
    return;
  }

  const result = await pool.query(
    'SELECT winner FROM player_matches INNER JOIN matches ON player_matches.match = matches.id WHERE player = $1',
    [player]
  );
  const totalMatches = result.rowCount;
  const wins = result.rows.map(r => r.winner).reduce((total, winner) => total + (winner === player ? 1 : 0), 0);
  res.write(`matches: ${totalMatches}\n`);
  res.write(`wins: ${wins}\n`);
  res.write(`losses: ${totalMatches - wins}\n`);
  res.write(`win percent: ${wins / totalMatches}\n`);
  res.end();
});

app.get('/scores/friends/:player_id', (req, res) => {
  res.end('not implemented');
});

const port = process.env.PORT || 3000;
app.listen(port, () => {
  console.log(`Listening on port ${port}.`);
});
