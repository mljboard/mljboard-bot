CREATE TABLE IF NOT EXISTS discord_pairing_codes (
	discord_username VARCHAR(45),
	pairing_code VARCHAR(60)
);


CREATE TABLE IF NOT EXISTS discord_websites (
	discord_username VARCHAR(45),
	website VARCHAR(2083)
);


CREATE TABLE IF NOT EXISTS lastfm_usernames (
	discord_username VARCHAR(45),
	lastfm_username VARCHAR(50)
);
