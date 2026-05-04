create table if not exists seed_history (
    name text primary key,
    checksum text not null,
    applied_at timestamptz not null default now()
);

create temporary table seed_roster_players (
    discord_user_id text primary key
) on commit drop;

insert into seed_roster_players (discord_user_id)
values
('298460716904611840'),
('193687671359406080'),
('341841825847574528'),
('307413040712777730'),
('230324826747699200'),
('526101639002587178'),
('360123145522249729'),
('257947077324177411'),
('234388219020443648'),
('486541449568059402'),
('1322931563406688346'),
('353863554585067532'),
('364448191724257280'),
('287674226440142848'),
('235195687996882954'),
('226071331697852416'),
('211134988119310337'),
('625411819334270976'),
('521657297693573120'),
('556582468002775050'),
('240514249024077824'),
('319831527821410316'),
('580756767148081152'),
('247320467310903297'),
('244543163702837259'),
('163288553923543041'),
('128246284447514624'),
('532245370944880640'),
('379260041066446849'),
('363760292913479680'),
('283222297807028224'),
('1002921317051420703'),
('817226698071998484'),
('431791718535528448'),
('252177261006946304'),
('961651735187841114');

create temporary table seed_roster_character_identities (
    name text not null,
    region text not null,
    server text not null
) on commit drop;

insert into seed_roster_character_identities (name, region, server)
values
('Lazykit', 'eu', 'spineshatter'),
('Notalazy', 'eu', 'spineshatter'),
('Palayoshka', 'eu', 'spineshatter'),
('Yoshkap', 'eu', 'spineshatter'),
('Kravá', 'eu', 'spineshatter'),
('Kravä', 'eu', 'spineshatter'),
('Kravarlocka', 'eu', 'spineshatter'),
('Cambels', 'eu', 'spineshatter'),
('Cambellsh', 'eu', 'spineshatter'),
('Bawownahunt', 'eu', 'spineshatter'),
('Fleabus', 'eu', 'spineshatter'),
('Dubasaw', 'eu', 'spineshatter'),
('Dubasw', 'eu', 'spineshatter'),
('Lansena', 'eu', 'spineshatter'),
('Mellorin', 'eu', 'spineshatter'),
('Edgelørd', 'eu', 'spineshatter'),
('Udy', 'eu', 'spineshatter'),
('Brokentoy', 'eu', 'spineshatter'),
('Zabyjaka', 'eu', 'spineshatter'),
('Entim', 'eu', 'spineshatter'),
('Entimx', 'eu', 'spineshatter'),
('Serjioo', 'eu', 'spineshatter'),
('Chaiser', 'eu', 'spineshatter'),
('Globaldoctor', 'eu', 'spineshatter'),
('Legalise', 'eu', 'spineshatter'),
('Tenten', 'eu', 'spineshatter'),
('Alpine', 'eu', 'spineshatter'),
('Dontmissbyte', 'eu', 'spineshatter'),
('Kyivstar', 'eu', 'spineshatter'),
('Hellpapa', 'eu', 'spineshatter'),
('Strikex', 'eu', 'spineshatter'),
('Swotchw', 'eu', 'spineshatter'),
('Swotchww', 'eu', 'spineshatter'),
('Lazyswotch', 'eu', 'spineshatter'),
('Artlicht', 'eu', 'spineshatter'),
('Llóydaustin', 'eu', 'spineshatter'),
('Pigtasty', 'eu', 'spineshatter'),
('Zulnani', 'eu', 'spineshatter'),
('Garhard', 'eu', 'spineshatter'),
('Bozer', 'eu', 'spineshatter'),
('Amorales', 'eu', 'spineshatter'),
('Greenbeam', 'eu', 'spineshatter'),
('Mapleleaf', 'eu', 'spineshatter'),
('Renko', 'eu', 'spineshatter'),
('Anyataylorj', 'eu', 'spineshatter'),
('Fantagodx', 'eu', 'spineshatter'),
('Pappit', 'eu', 'spineshatter'),
('Puppit', 'eu', 'spineshatter'),
('Thundermate', 'eu', 'spineshatter'),
('Blinkmate', 'eu', 'spineshatter'),
('Rinn', 'eu', 'spineshatter'),
('Figurehead', 'eu', 'spineshatter'),
('Chortenya', 'eu', 'spineshatter'),
('Sypukha', 'eu', 'spineshatter'),
('Ryzhã', 'eu', 'spineshatter'),
('Ryzha', 'eu', 'spineshatter'),
('Alarian', 'eu', 'spineshatter'),
('Romovababka', 'eu', 'spineshatter'),
('Betonnyi', 'eu', 'spineshatter'),
('Dullo', 'eu', 'spineshatter'),
('Manowarr', 'eu', 'spineshatter'),
('Prõfessõr', 'eu', 'spineshatter'),
('Benzöbak', 'eu', 'spineshatter'),
('Flinesham', 'eu', 'spineshatter'),
('Skifon', 'eu', 'spineshatter'),
('Monkablaze', 'eu', 'spineshatter'),
('Monkablazé', 'eu', 'spineshatter'),
('Deserve', 'eu', 'spineshatter'),
('Deserv', 'eu', 'spineshatter');

delete from characters c
using seed_roster_character_identities sc
where lower(trim(c.name)) = lower(trim(sc.name))
  and lower(trim(c.region)) = lower(trim(sc.region))
  and lower(trim(c.server)) = lower(trim(sc.server));

delete from guild_memberships gm
using discord_identities di, seed_roster_players sp
where gm.guild_id = 'a1f1bcf2-47c2-4d57-bf9e-24edd3a6dcae'
  and gm.user_id = di.user_id
  and di.discord_user_id = sp.discord_user_id
  and not exists (
      select 1
      from characters c
      where c.user_id = gm.user_id
        and c.deleted_at is null
  );

create temporary table seed_roster_identity_cleanup (
    identity_id uuid primary key,
    user_id uuid not null
) on commit drop;

insert into seed_roster_identity_cleanup (identity_id, user_id)
select di.id, di.user_id
from discord_identities di
join seed_roster_players sp on sp.discord_user_id = di.discord_user_id
where not exists (select 1 from characters c where c.user_id = di.user_id)
  and not exists (select 1 from guild_memberships gm where gm.user_id = di.user_id)
  and not exists (select 1 from discord_profiles dp where dp.discord_identity_id = di.id)
  and not exists (select 1 from discord_oauth_tokens dot where dot.discord_identity_id = di.id)
  and not exists (select 1 from user_sessions us where us.user_id = di.user_id)
  and not exists (select 1 from security_audit_events sae where sae.user_id = di.user_id);

delete from discord_identities di
using seed_roster_identity_cleanup cleanup
where di.id = cleanup.identity_id;

delete from users u
using seed_roster_identity_cleanup cleanup
where u.id = cleanup.user_id
  and not exists (select 1 from discord_identities di where di.user_id = u.id)
  and not exists (select 1 from characters c where c.user_id = u.id)
  and not exists (select 1 from guild_memberships gm where gm.user_id = u.id)
  and not exists (select 1 from user_sessions us where us.user_id = u.id)
  and not exists (select 1 from security_audit_events sae where sae.user_id = u.id);

delete from seed_history
where name = '20260504_character_roster';
