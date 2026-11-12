-- Vodka parties: a group of up to six people who chat and launch games together.
-- Reachable only through the `vodka-party` edge function (service role).

create table public.vodka_parties (
  id bigint generated always as identity primary key,
  leader bigint not null references public.vodka_users(roblox_id) on delete cascade,
  created_at timestamptz not null default now()
);
create index vodka_parties_leader on public.vodka_parties(leader);

create table public.vodka_party_members (
  party_id bigint not null references public.vodka_parties(id) on delete cascade,
  roblox_id bigint not null unique references public.vodka_users(roblox_id) on delete cascade,
  joined_at timestamptz not null default now(),
  primary key (party_id, roblox_id)
);

create table public.vodka_party_invites (
  party_id bigint not null references public.vodka_parties(id) on delete cascade,
  roblox_id bigint not null references public.vodka_users(roblox_id) on delete cascade,
  invited_by bigint not null references public.vodka_users(roblox_id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (party_id, roblox_id)
);
create index vodka_party_invites_user on public.vodka_party_invites(roblox_id);
create index vodka_party_invites_by on public.vodka_party_invites(invited_by);

create table public.vodka_party_events (
  id bigint generated always as identity primary key,
  party_id bigint not null references public.vodka_parties(id) on delete cascade,
  sender bigint,
  kind text not null check (kind in ('text', 'launch', 'system')),
  body text not null check (char_length(body) <= 1000),
  payload jsonb,
  created_at timestamptz not null default now()
);
create index vodka_party_events_party on public.vodka_party_events(party_id, id);

alter table public.vodka_parties enable row level security;
alter table public.vodka_party_members enable row level security;
alter table public.vodka_party_invites enable row level security;
alter table public.vodka_party_events enable row level security;

revoke all on public.vodka_parties, public.vodka_party_members, public.vodka_party_invites, public.vodka_party_events from anon, authenticated;
