# ContainerDesk — Codex paket sa praćenjem

**60 numerisanih promptova za Tauri desktop aplikaciju za Docker preko SSH.**

Radni naziv: **ContainerDesk**. Naziv je interni predlog; dostupnost domena/žiga nije proveravana.
Verzija paketa: 1.0.0 • Datum: 28.09.2026.

Ovo je paket za izradu aplikacije u lokalnom Codex projektu. Sadrži detaljne zadatke, arhitektonske odluke, kriterijume provere i funkcionalan tracker. Izvorni kod aplikacije i instalacioni paketi nastaju izvršavanjem promptova.

## Šta gradimo

- Jednu Tauri v2 desktop aplikaciju: React + TypeScript + Vite + Tailwind, Rust + Tokio.
- Linux i macOS klijente sa ugrađenim interfejsom za Docker na udaljenim Linux instancama.
- Povezivanje preko lokalnog OpenSSH klijenta i postojećeg `~/.ssh/config`, uključujući `ProxyJump`, `IdentityFile` i SSH agent.
- Liste servera/kontejnera, detalje, health, portove, logove uživo, CPU/RAM/I/O i Docker događaje.
- Start/stop/restart, ograničene grupne akcije, Compose projekte, pregled slika, mreža i volumena.
- Terminal u kontejneru sa pravim PTY-jem, prekid veze i čišćenje procesa.
- Linux pakete i macOS app/DMG, CI konfiguraciju i proveru stvarne kompatibilnosti.

Na računaru koji koristi završenu aplikaciju **nisu potrebni Docker, Docker Desktop, jq, Python, Node ili Rust**. Potrebni su OpenSSH, ispravan pristup serveru i odgovarajuće sistemske biblioteke za Tauri paket. Python 3.10+ se koristi tokom razvoja samo za tracker. Za izradu aplikacije potrebni su Rust, Node i sistemski razvojni paketi.

## Brzi početak

1. Raspakuj ZIP u folder u kome želiš aplikaciju.
2. Otvori taj folder u svom Codex okruženju.
3. Pročitaj `CODEX_START.md` i pošalji početni tekst iz tog fajla.
4. Izvršavaj po jedan prompt. Tracker određuje sledeći korak.

Iz korena projekta:

```bash
python3 codex/scripts/track.py validate
python3 codex/scripts/track.py next
python3 codex/scripts/track.py show 001
```

Prvi prompt pregleda repozitorijum i okruženje. Drugi kreira Tauri osnovu u istom korenu projekta.

Ako već imaš repozitorijum, prvo kopiraj `codex/`, `CODEX_START.md` i ovaj README. Ako repozitorijum već ima `AGENTS.md`, **ručno spoji relevantna pravila**, bez prepisivanja postojećeg fajla. Ne prepisuj postojeći `codex/` ako već sadrži tvoj rad; prvo napravi lokalnu kopiju i prilagodi putanje.

## Početna poruka za Codex

```text
Pročitaj AGENTS.md, CODEX_START.md i codex/docs/ARCHITECTURE.md.
Pokreni python3 codex/scripts/track.py next.
Pronađi i izvrši samo taj prompt, poštujući zavisnosti i postojeći kod.
Pre rada postavi status start. Implementiraj konkretan rezultat i proveri ga.
Upiši stvarne dokaze u codex/tracking/evidence/NNN.md.
Označi done samo ako su kriterijumi završetka ispunjeni; inače block sa razlogom.
Preporuči reasoning iz manifesta. Ako ne možeš da menjaš model/reasoning,
nemoj tvrditi da si ga promenio. Na kraju prikaži rezultat i sledeći korak.
```

Za nastavak u novoj sesiji pošalji isti tekst. `next` vraća aktivan/blokiran korak pre nego što ponudi novi.

## Faze

| Promptovi | Rezultat |
|---|---|
| 001–008 | Osnova aplikacije, IPC, podešavanja, pravila i test podaci |
| 009–018 | SSH config, jump hostovi, ključevi, sesije i stvarna provera veze |
| 019–030 | MVP za pregled: kontejneri, logovi, statistika, Compose grupe |
| 031–038 | Upravljanje kontejnerima i pregled ostalih Docker resursa |
| 039–046 | Terminal, oporavak veze, dijagnostika i kompletan interfejs |
| 047–060 | Provere, pakovanje, CI, Linux/macOS prihvat i predaja |

Prvi upotrebljiv MVP za pregled je posle **030**. Upravljanje je završeno do **038**, sve funkcije do **046**, a poslednjih 14 koraka proverava i priprema isporuku.

## Šta prati tracker

- `pending`, `in_progress`, `blocked`, `done` za svaki prompt.
- Preporučeni reasoning: `medium` ili `high`, prema vrsti zadatka.
- Zavisnosti: svaki korak traži završen prethodni korak.
- Stvarne komande/provere, putanju do dokaza i njihov SHA-256.
- Istoriju promena, vreme početka/završetka i opcioni Git commit.
- Samo jedan aktivan prompt; promene se upisuju atomski uz lokalno zaključavanje.

Tracker automatski određuje sledeći korak i prikazuje preporuku za reasoning. **Ne pokreće Codex, ne poziva model API i ne može sam da promeni reasoning podešavanje u tvom editoru.**

## Važne granice

- Aplikacija koristi SSH identitete i pristup koje već imaš. Ne instalira servis na serveru.
- Čita postojeći SSH config; ne menja ga automatski.
- Ciljni server je Linux sa Docker Engine-om i podržanim POSIX shell-om.
- Docker pristup se proverava za udaljenog korisnika, uključujući opcioni postojeći `sudo -n` režim.
- Read-only režim je zaštita unutar aplikacije, ne zamena za dozvole na serveru.
- Instalacija i javna distribucija na macOS-u imaju odvojene provere potpisa/notarizacije. Bez Apple pristupa paket ne tvrdi da je javno potpisivanje provereno.
- Izvršavanje promptova 018/030 i drugih integracionih provera zahteva tvoje test okruženje. Linux i macOS runtime provere moraju zaista da se izvrše; izmišljeni prolaz nije prihvatljiv.

## Sadržaj

| Putanja | Namena |
|---|---|
| `CODEX_START.md` | Početni i nastavak prompt |
| `AGENTS.md` | Pravila rada u repozitorijumu |
| `codex/prompts/` | 60 zasebnih kompletnih promptova |
| `codex/ALL_PROMPTS.md` | Svi promptovi u jednom fajlu |
| `codex/manifest.json` | ID, redosled, zavisnosti, reasoning i hash promptova |
| `codex/tracking/state.json` | Izvor istine za napredak |
| `codex/tracking/TRACKER.md` | Čitljiv pregled i istorija |
| `codex/scripts/track.py` | Funkcionalan lokalni tracker |
| `codex/docs/` | Arhitektura, plan, komande, platforme i izvori |
| `codex/examples/ssh-config.example` | Primer direktne i jump veze |
| `codex/tests/test_tracker.py` | Provere tracker-a bez SSH/mreže |
| `PACK_VERIFICATION.md` | Šta je provereno pri izradi ovog paketa |

Detaljne komande: `codex/docs/TRACKING.md`.
