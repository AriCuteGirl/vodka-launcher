-- Vodka Launcher chat. Every table is reachable only through the `vodka` edge function
-- (service role). Row level security is on with no policies, so the public API keys see nothing.

create table public.vodka_users (
  roblox_id bigint primary key,
  name text not null,
  display_name text not null,
  code text not null unique,
  created_at timestamptz not null default now(),
  last_seen timestamptz not null default now()
);

create table public.vodka_verifications (
  roblox_id bigint primary key,
  challenge text not null,
  attempts int not null default 0,
  expires_at timestamptz not null
);

create table public.vodka_sessions (
  token_hash text primary key,
  roblox_id bigint not null references public.vodka_users(roblox_id) on delete cascade,
  created_at timestamptz not null default now(),
  last_used timestamptz not null default now()
);
create index vodka_sessions_user on public.vodka_sessions(roblox_id);

create table public.vodka_contacts (
  owner bigint not null references public.vodka_users(roblox_id) on delete cascade,
  other bigint not null references public.vodka_users(roblox_id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (owner, other),
  check (owner <> other)
);
create index vodka_contacts_other on public.vodka_contacts(other);

create table public.vodka_blocks (
  owner bigint not null references public.vodka_users(roblox_id) on delete cascade,
  blocked bigint not null references public.vodka_users(roblox_id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (owner, blocked)
);
create index vodka_blocks_blocked on public.vodka_blocks(blocked);

create table public.vodka_messages (
  id bigint generated always as identity primary key,
  sender bigint not null references public.vodka_users(roblox_id) on delete cascade,
  recipient bigint not null references public.vodka_users(roblox_id) on delete cascade,
  kind text not null check (kind in ('text', 'invite')),
  body text not null check (char_length(body) <= 1000),
  payload jsonb,
  created_at timestamptz not null default now()
);
create index vodka_messages_recipient on public.vodka_messages(recipient, id);
create index vodka_messages_sender on public.vodka_messages(sender, id);
create index vodka_messages_created on public.vodka_messages(created_at);

create table public.vodka_reports (
  id bigint generated always as identity primary key,
  reporter bigint not null references public.vodka_users(roblox_id) on delete cascade,
  reported bigint not null,
  reason text not null check (char_length(reason) <= 500),
  created_at timestamptz not null default now()
);
create index vodka_reports_reporter on public.vodka_reports(reporter);

alter table public.vodka_users enable row level security;
alter table public.vodka_verifications enable row level security;
alter table public.vodka_sessions enable row level security;
alter table public.vodka_contacts enable row level security;
alter table public.vodka_blocks enable row level security;
alter table public.vodka_messages enable row level security;
alter table public.vodka_reports enable row level security;

revoke all on public.vodka_users, public.vodka_verifications, public.vodka_sessions, public.vodka_contacts,
  public.vodka_blocks, public.vodka_messages, public.vodka_reports from anon, authenticated;
