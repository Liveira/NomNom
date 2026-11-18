/** The sites the extension works on. The manifest lists the same hosts (a test keeps them equal), and
 *  the NomNom app accepts the same ones. */
export const DEDICATED_SITES = ['youtube.com', 'x.com', 'twitter.com', 'instagram.com', 'reddit.com'];

/** Sites that get a button on top of their video player. */
export const PLAYER_SITES = [
  'tiktok.com',
  'vimeo.com',
  'twitch.tv',
  'dailymotion.com',
  'facebook.com',
  'bilibili.com',
  'streamable.com',
  'bsky.app',
  'tumblr.com',
  'pinterest.com',
  'imgur.com',
];

export const ALL_SITES = [...DEDICATED_SITES, ...PLAYER_SITES];
