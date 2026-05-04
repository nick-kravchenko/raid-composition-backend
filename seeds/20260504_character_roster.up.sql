do $$
begin
    if not exists (
        select 1
        from guilds
        where id = 'a1f1bcf2-47c2-4d57-bf9e-24edd3a6dcae'
          and deleted_at is null
    ) then
        raise exception 'target guild a1f1bcf2-47c2-4d57-bf9e-24edd3a6dcae was not found or is deleted';
    end if;
end $$;

create temporary table seed_roster_players (
    discord_user_id text primary key,
    player_name text not null,
    user_id uuid not null default gen_random_uuid()
) on commit drop;

insert into seed_roster_players (discord_user_id, player_name)
values
('298460716904611840', 'Lazykit'),
('193687671359406080', 'Palayoshka'),
('341841825847574528', 'Krava'),
('307413040712777730', 'Cambells'),
('230324826747699200', 'Bawowna'),
('526101639002587178', 'Dubasaw'),
('360123145522249729', 'Lansena'),
('257947077324177411', 'Edgelord'),
('234388219020443648', 'Brokentoy'),
('486541449568059402', 'Entim'),
('1322931563406688346', 'Serjio'),
('353863554585067532', 'Chaiser'),
('364448191724257280', 'Legalise'),
('287674226440142848', 'Dontmissbyte'),
('235195687996882954', 'Strike'),
('226071331697852416', 'Swotch'),
('211134988119310337', 'Art'),
('625411819334270976', 'Lloydaustin'),
('521657297693573120', 'Zulnani'),
('556582468002775050', 'Amorales'),
('240514249024077824', 'Greenbeam'),
('319831527821410316', 'Renko'),
('580756767148081152', 'Pappit'),
('247320467310903297', 'Thundermate'),
('244543163702837259', 'Rinn'),
('163288553923543041', 'Chortenya'),
('128246284447514624', 'Ryzha'),
('532245370944880640', 'Alarian'),
('379260041066446849', 'Sustainable'),
('363760292913479680', 'Dullo'),
('283222297807028224', 'Manowarr'),
('1002921317051420703', 'Professor'),
('817226698071998484', 'Benzobak'),
('431791718535528448', 'Fline'),
('252177261006946304', 'Nevercome'),
('961651735187841114', 'Deserve');

update seed_roster_players p
set user_id = di.user_id
from discord_identities di
where di.discord_user_id = p.discord_user_id;

insert into users (id)
select p.user_id
from seed_roster_players p
left join discord_identities di on di.discord_user_id = p.discord_user_id
where di.id is null
on conflict (id) do nothing;

insert into discord_identities (user_id, discord_user_id)
select p.user_id, p.discord_user_id
from seed_roster_players p
on conflict (discord_user_id) do nothing;

update seed_roster_players p
set user_id = di.user_id
from discord_identities di
where di.discord_user_id = p.discord_user_id;

with latest_deleted_membership as (
    select distinct on (gm.user_id) gm.id
    from guild_memberships gm
    join seed_roster_players p on p.user_id = gm.user_id
    where gm.guild_id = 'a1f1bcf2-47c2-4d57-bf9e-24edd3a6dcae'
      and gm.deleted_at is not null
      and not exists (
          select 1
          from guild_memberships active_gm
          where active_gm.guild_id = gm.guild_id
            and active_gm.user_id = gm.user_id
            and active_gm.deleted_at is null
      )
    order by gm.user_id, gm.created_at desc
)
update guild_memberships gm
set role = 'raider',
    deleted_at = null
from latest_deleted_membership latest
where gm.id = latest.id;

insert into guild_memberships (guild_id, user_id, role)
select 'a1f1bcf2-47c2-4d57-bf9e-24edd3a6dcae', p.user_id, 'raider'
from seed_roster_players p
where not exists (
    select 1
    from guild_memberships gm
    where gm.guild_id = 'a1f1bcf2-47c2-4d57-bf9e-24edd3a6dcae'
      and gm.user_id = p.user_id
      and gm.deleted_at is null
)
on conflict do nothing;

create temporary table seed_roster_characters (
    discord_user_id text not null,
    name text not null,
    region text not null,
    server text not null,
    class text not null,
    race text not null,
    spec text not null,
    role text not null,
    rank text not null
) on commit drop;

insert into seed_roster_characters (discord_user_id, name, region, server, class, race, spec, role, rank)
values
('298460716904611840', 'Lazykit', 'eu', 'spineshatter', 'shaman', 'Orc', 'Enhancement', 'melee', 'main'),
('298460716904611840', 'Notalazy', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Retribution', 'melee', 'alt'),
('193687671359406080', 'Palayoshka', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Holy', 'heal', 'main'),
('193687671359406080', 'Yoshkap', 'eu', 'spineshatter', 'priest', 'Undead', 'Holy', 'heal', 'alt'),
('341841825847574528', 'Kravá', 'eu', 'spineshatter', 'warrior', 'Orc', 'Fury', 'melee', 'main'),
('341841825847574528', 'Kravä', 'eu', 'spineshatter', 'hunter', 'Orc', 'BeastMastery', 'ranged', 'alt'),
('341841825847574528', 'Kravarlocka', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'alt'),
('307413040712777730', 'Cambels', 'eu', 'spineshatter', 'mage', 'Troll', 'Arcane', 'ranged', 'main'),
('307413040712777730', 'Cambellsh', 'eu', 'spineshatter', 'hunter', 'Orc', 'Survival', 'ranged', 'alt'),
('230324826747699200', 'Bawownahunt', 'eu', 'spineshatter', 'hunter', 'Orc', 'BeastMastery', 'ranged', 'main'),
('230324826747699200', 'Fleabus', 'eu', 'spineshatter', 'druid', 'Tauren', 'Restoration', 'heal', 'alt'),
('526101639002587178', 'Dubasaw', 'eu', 'spineshatter', 'hunter', 'Orc', 'BeastMastery', 'ranged', 'main'),
('526101639002587178', 'Dubasw', 'eu', 'spineshatter', 'shaman', 'Orc', 'Enhancement', 'melee', 'alt'),
('360123145522249729', 'Lansena', 'eu', 'spineshatter', 'priest', 'Troll', 'Holy', 'heal', 'main'),
('360123145522249729', 'Mellorin', 'eu', 'spineshatter', 'druid', 'Tauren', 'Balance', 'ranged', 'alt'),
('257947077324177411', 'Edgelørd', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'main'),
('257947077324177411', 'Udy', 'eu', 'spineshatter', 'priest', 'Undead', 'Shadow', 'ranged', 'alt'),
('234388219020443648', 'Brokentoy', 'eu', 'spineshatter', 'shaman', 'Orc', 'Enhancement', 'melee', 'main'),
('234388219020443648', 'Zabyjaka', 'eu', 'spineshatter', 'shaman', 'Troll', 'Elemental', 'ranged', 'alt'),
('486541449568059402', 'Entim', 'eu', 'spineshatter', 'druid', 'Tauren', 'Restoration', 'heal', 'main'),
('486541449568059402', 'Entimx', 'eu', 'spineshatter', 'priest', 'Undead', 'Holy', 'heal', 'alt'),
('1322931563406688346', 'Serjioo', 'eu', 'spineshatter', 'hunter', 'Orc', 'Survival', 'ranged', 'main'),
('353863554585067532', 'Chaiser', 'eu', 'spineshatter', 'druid', 'Tauren', 'Feral', 'tank', 'main'),
('353863554585067532', 'Globaldoctor', 'eu', 'spineshatter', 'shaman', 'Troll', 'Restoration', 'heal', 'alt'),
('364448191724257280', 'Legalise', 'eu', 'spineshatter', 'hunter', 'Troll', 'BeastMastery', 'ranged', 'main'),
('364448191724257280', 'Tenten', 'eu', 'spineshatter', 'druid', 'Tauren', 'Feral', 'tank', 'alt'),
('364448191724257280', 'Alpine', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Protection', 'tank', 'alt'),
('287674226440142848', 'Dontmissbyte', 'eu', 'spineshatter', 'shaman', 'Troll', 'Elemental', 'ranged', 'main'),
('287674226440142848', 'Kyivstar', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'alt'),
('235195687996882954', 'Hellpapa', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'main'),
('235195687996882954', 'Strikex', 'eu', 'spineshatter', 'rogue', 'Orc', 'Combat', 'melee', 'alt'),
('226071331697852416', 'Swotchw', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'main'),
('226071331697852416', 'Swotchww', 'eu', 'spineshatter', 'warrior', 'Orc', 'Arms', 'melee', 'alt'),
('226071331697852416', 'Lazyswotch', 'eu', 'spineshatter', 'druid', 'Tauren', 'Restoration', 'heal', 'alt'),
('211134988119310337', 'Artlicht', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Retribution', 'melee', 'main'),
('625411819334270976', 'Llóydaustin', 'eu', 'spineshatter', 'warrior', 'Orc', 'Fury', 'melee', 'main'),
('625411819334270976', 'Pigtasty', 'eu', 'spineshatter', 'hunter', 'Orc', 'BeastMastery', 'ranged', 'alt'),
('521657297693573120', 'Zulnani', 'eu', 'spineshatter', 'druid', 'Tauren', 'Feral', 'tank', 'main'),
('521657297693573120', 'Garhard', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'alt'),
('556582468002775050', 'Bozer', 'eu', 'spineshatter', 'warrior', 'Orc', 'Arms', 'melee', 'main'),
('556582468002775050', 'Amorales', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'alt'),
('240514249024077824', 'Greenbeam', 'eu', 'spineshatter', 'shaman', 'Troll', 'Restoration', 'heal', 'main'),
('240514249024077824', 'Mapleleaf', 'eu', 'spineshatter', 'druid', 'Tauren', 'Balance', 'ranged', 'alt'),
('319831527821410316', 'Renko', 'eu', 'spineshatter', 'rogue', 'Troll', 'Combat', 'melee', 'main'),
('319831527821410316', 'Anyataylorj', 'eu', 'spineshatter', 'druid', 'Tauren', 'Feral', 'tank', 'alt'),
('319831527821410316', 'Fantagodx', 'eu', 'spineshatter', 'mage', 'Undead', 'Arcane', 'ranged', 'alt'),
('580756767148081152', 'Pappit', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Retribution', 'melee', 'main'),
('580756767148081152', 'Puppit', 'eu', 'spineshatter', 'shaman', 'Orc', 'Enhancement', 'melee', 'alt'),
('247320467310903297', 'Thundermate', 'eu', 'spineshatter', 'shaman', 'Orc', 'Enhancement', 'melee', 'main'),
('247320467310903297', 'Blinkmate', 'eu', 'spineshatter', 'mage', 'Troll', 'Arcane', 'ranged', 'alt'),
('244543163702837259', 'Rinn', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Protection', 'tank', 'main'),
('244543163702837259', 'Figurehead', 'eu', 'spineshatter', 'shaman', 'Troll', 'Restoration', 'heal', 'alt'),
('163288553923543041', 'Chortenya', 'eu', 'spineshatter', 'warlock', 'Orc', 'Destruction', 'ranged', 'main'),
('163288553923543041', 'Sypukha', 'eu', 'spineshatter', 'druid', 'Tauren', 'Balance', 'ranged', 'alt'),
('128246284447514624', 'Ryzhã', 'eu', 'spineshatter', 'priest', 'Troll', 'Holy', 'heal', 'main'),
('128246284447514624', 'Ryzha', 'eu', 'spineshatter', 'priest', 'Troll', 'Holy', 'heal', 'alt'),
('532245370944880640', 'Alarian', 'eu', 'spineshatter', 'priest', 'Undead', 'Shadow', 'ranged', 'main'),
('532245370944880640', 'Romovababka', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Holy', 'heal', 'alt'),
('379260041066446849', 'Betonnyi', 'eu', 'spineshatter', 'warrior', 'Undead', 'Arms', 'melee', 'main'),
('363760292913479680', 'Dullo', 'eu', 'spineshatter', 'shaman', 'Night Elf', 'Enhancement', 'melee', 'main'),
('283222297807028224', 'Manowarr', 'eu', 'spineshatter', 'hunter', 'Orc', 'BeastMastery', 'ranged', 'main'),
('1002921317051420703', 'Prõfessõr', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Protection', 'tank', 'main'),
('817226698071998484', 'Benzöbak', 'eu', 'spineshatter', 'druid', 'Tauren', 'Balance', 'ranged', 'main'),
('431791718535528448', 'Flinesham', 'eu', 'spineshatter', 'shaman', 'Orc', 'Elemental', 'ranged', 'main'),
('431791718535528448', 'Skifon', 'eu', 'spineshatter', 'mage', 'Troll', 'Arcane', 'ranged', 'alt'),
('252177261006946304', 'Monkablaze', 'eu', 'spineshatter', 'mage', 'Troll', 'Arcane', 'ranged', 'main'),
('252177261006946304', 'Monkablazé', 'eu', 'spineshatter', 'paladin', 'Blood Elf', 'Protection', 'tank', 'alt'),
('961651735187841114', 'Deserve', 'eu', 'spineshatter', 'hunter', 'Troll', 'BeastMastery', 'ranged', 'main'),
('961651735187841114', 'Deserv', 'eu', 'spineshatter', 'druid', 'Tauren', 'Feral', 'tank', 'alt');

with matched_characters as (
    select distinct on (
        lower(trim(sc.name)),
        lower(trim(sc.region)),
        lower(trim(sc.server))
    )
        c.id,
        p.user_id,
        sc.name,
        sc.region,
        sc.server,
        sc.class,
        sc.race,
        sc.spec,
        sc.role,
        sc.rank
    from seed_roster_characters sc
    join seed_roster_players p on p.discord_user_id = sc.discord_user_id
    join characters c
      on lower(trim(c.name)) = lower(trim(sc.name))
     and lower(trim(c.region)) = lower(trim(sc.region))
     and lower(trim(c.server)) = lower(trim(sc.server))
    order by lower(trim(sc.name)), lower(trim(sc.region)), lower(trim(sc.server)),
             (c.deleted_at is null) desc, c.created_at desc
)
update characters c
set user_id = matched.user_id,
    name = matched.name,
    region = matched.region,
    server = matched.server,
    class = matched.class,
    race = matched.race,
    spec = matched.spec,
    role = matched.role,
    rank = matched.rank,
    deleted_at = null
from matched_characters matched
where c.id = matched.id;

insert into characters (user_id, name, region, server, class, race, spec, role, rank)
select p.user_id, sc.name, sc.region, sc.server, sc.class, sc.race, sc.spec, sc.role, sc.rank
from seed_roster_characters sc
join seed_roster_players p on p.discord_user_id = sc.discord_user_id
where not exists (
    select 1
    from characters c
    where lower(trim(c.name)) = lower(trim(sc.name))
      and lower(trim(c.region)) = lower(trim(sc.region))
      and lower(trim(c.server)) = lower(trim(sc.server))
);
