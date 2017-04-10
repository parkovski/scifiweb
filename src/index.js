import express from 'express';
import { Pool } from 'pg';
import Ajv from 'ajv';

const app = express();
const pool = new Pool({
  user: 'scifi',
  password: 'scifi',
  host: 'localhost',
  database: 'scifi',
  max: 10,
  idleTimeoutMillis: 5000,
});

const ajv = new Ajv();

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

function authorize(req, res, next) {
  if (req.query.auth === 'secret') {
    next();
  } else {
    res.writeHead(403);
    res.end('Not authorized');
  }
}

app.get('/', (req, res) => {
  res.end('I bet you\'ll find this site super useful.');
});

const validate_match_new = ajv.compile({
  type: 'array',
  minItems: 1,
  maxItems: 4,
  items: {
    properties: {
      id: {
        type: 'number',
        minimum: 0,
      },
      kills: {
        type: 'number',
        minimum: 0,
      },
      deaths: {
        type: 'number',
        minimum: 0,
      },
    }
  }
});
/// params: auth=x, players=[{id, kills, deaths}], winner=id
app.post('/match/new', authorize, async (req, res) => {
  let players, winner;
  try {
    players = JSON.parse(req.query.players);
    if (!validate_match_new(players)) {
      res.writeHead(400);
      res.end('Invalid player data format');
      return;
    }
    winner = validateOnePlayerId(req.query.winner);
  } catch (e) {
    res.writeHead(400);
    res.end('Malformed data');
    return;
  }

  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    const result = await client.query(
      'INSERT INTO matches (winner) VALUES ($1) RETURNING id',
      [winner]
    );
    const matchId = result.rows[0].id;
    for (const player of players) {
      await client.query(
        'INSERT INTO player_matches (player, match, kills, deaths) VALUES ($1, $2, $3, $4)',
        [player.id, matchId, player.kills, player.deaths]
      );
      let update = 'UPDATE players SET matches = matches + 1';
      if (player.id === winner) {
        update += ', wins = wins + 1';
      }
      update += ', kills = kills + $1, deaths = deaths + $2 WHERE id = $3';
      await client.query(update, [player.kills, player.deaths, player.id]);
    }
    await client.query('COMMIT');
    res.write('created match ' + matchId);
  } catch (e) {
    res.writeHead(500);
    res.write('match creation failed');
    console.log(e);
  } finally {
    client.release();
    res.end();
  }
});

/// params: auth, name, nickname
app.post('/player/new', authorize, async (req, res) => {
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
    res.writeHead(500);
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

app.get('/player/:player_id/stats/competitors', async (req, res) => {
  let player;
  try {
    player = validateOnePlayerId(req.params.player_id);
  } catch (e) {
    res.writeHead(400, "Player ID is invalid.");
    res.end();
  }

  const result = await pool.query(
    'SELECT DISTINCT p.id, p.name, p.matches, p.wins, p.kills, p.deaths FROM players p'
    + ' JOIN player_matches pm1 ON pm1.player = p.id'
    + ' JOIN player_matches pm2 ON pm1.match = pm2.match'
    + ' WHERE pm2.player = $1',
    [player]
  );

  if (result.rowCount === 0) {
    res.end('[]');
    return;
  }

  res.end(JSON.stringify(result.rows));
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
