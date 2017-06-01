import authorize from './authorize';

import { Router } from 'express';
import Ajv from 'ajv';

const ajv = new Ajv();

const validateNewMatchData = ajv.compile({
  type: 'array',
  minItems: 1,
  maxItems: 4,
  items: {
    properties: {
      id: {
        type: 'number',
        minimum: -1,
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

export default function matchRouter(pool) {
  const router = Router();

  /// Params: players, winner, auth
  router.post('/new', authorize, async (req, res) => {
    let players, winner;
    try {
      players = JSON.parse(req.query.players);
      winner = parseInt(req.query.winner);
      if (!validateNewMatchData(players)) {
        res.writeHead(400);
        res.end('Invalid player data');
        return;
      } else if (isNaN(winner) || winner < 0) {
        res.writeHead(400);
        res.end('Invalid winner ID');
        return;
      }
    } catch (e) {
      res.writeHead(400);
      res.end('Invalid player data');
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
        // Skip players we know don't exist.
        if (player.id === -1) {
          continue;
        }

        // Create a record of this player in this match.
        await client.query(
          'INSERT INTO player_matches (player, match, kills, deaths) VALUES ($1, $2, $3, $4)',
          [player.id, matchId, player.kills, player.deaths]
        );

        // Update the player's kill, death, and win stats.
        let updatePlayer = 'UPDATE players SET matches = matches + 1';
        if (player.id === winner) {
          updatePlayer += ', wins = wins + 1';
        }
        updatePlayer += ', kills = kills + $1, deaths = deaths + $2 WHERE id = $3';
        await client.query(updatePlayer, [player.kills, player.deaths, player.id]);
      }
      await client.query('COMMIT');
      res.write('Match ID: ' + matchId);
    } catch (e) {
      res.writeHead(500);
      res.write('Match creation failed');
      console.log(e);
    } finally {
      client.release();
      res.end();
    }
  });

  return router;
};