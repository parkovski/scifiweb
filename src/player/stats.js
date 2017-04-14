import { Router } from 'express';

export default function statsRouter(pool) {
  const router = Router({ mergeParams: true });

  // This router is mounted after player ID validation.
  router.get('/competitors', async (req, res) => {
    const player = req.params.player_id;
    const result = await pool.query(
      'SELECT DISTINCT ON (p.id) p.id, p.name, p.matches, p.wins, p.kills, p.deaths FROM players p'
      + ' JOIN player_matches pm1 ON pm1.player = p.id'
      + ' JOIN player_matches pm2 ON pm1.match = pm2.match'
      + ' WHERE pm2.player = $1',
      [player]
    );

    if (result.rowCount === 0) {
      res.end('[]');
    } else {
      res.end(JSON.stringify(result.rows));
    }
  });

  router.get('/friends', async (req, res) => {
    //const player = req.params.player_id;
    res.end('[]');
  });

  return router;
};