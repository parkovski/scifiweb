import statsRouter from './stats';
import authorize from '../authorize';

import { Router } from 'express';
import Ajv from 'ajv';

function validatePlayerId(req, res, next) {
  const id = parseInt(req.params.player_id);
  if (isNaN(id) || id <= 0) {
    res.writeHead(400);
    res.end('Invalid player ID.');
  } else {
    next();
  }
}

export default function playerRouter(pool) {
  const router = Router({ mergeParams: true });

  router.use('/:player_id/stats', validatePlayerId, statsRouter(pool));

  router.get('/id-for-fbid/:fbid', async (req, res) => {
    const fbid = req.params.fbid;
    const result = await pool.query('SELECT id FROM facebook_users WHERE fbid = $1', [fbid]);
    if (result.rowCount === 0) {
      res.writeHead(404);
      res.end();
    } else {
      res.end(result.rows[0].id.toString());
    }
  });

  router.get('/new', authorize, async (req, res) => {
    const name = req.query.name;
    const nickname = req.query.nickname;
    try {
      const result = await pool.query(
        'INSERT INTO players (name, nickname) VALUES ($1, $2) RETURNING id',
        [name, nickname]
      );
      res.end('Player ID: ' + result.rows[0].id);
    } catch (e) {
      res.writeHead(500);
      res.end('Error creating player');
      console.log(e);
    }
  });

  return router;
};