# ContainerDesk

Desktop aplikacija za rad sa Docker-om na Linux serverima preko tvog postojećeg OpenSSH pristupa. Klijenti su Linux i macOS; lokalni Docker/Docker Desktop, Node, Rust i Python nisu potrebni za korišćenje aplikacije. Potrebni su lokalni OpenSSH, sistemske biblioteke paketa i postojeći pristup udaljenom Docker Engine-u.

Aplikacija prikazuje kontejnere, inspect detalje, logove, statistiku, Compose projekte, slike, volumene i mreže. Start/stop/restart, ograničeno uklanjanje zaustavljenih kontejnera i terminal traže posebno omogućavanje i potvrdu. Slike, volumeni i mreže su samo za pregled. Nema prune-a, Compose up/down, registry prijavljivanja, Kubernetes-a ili automatskog ažuriranja. Radni naziv je ContainerDesk; nije tvrdnja o registrovanom žigu.

**Status:** funkcije su implementirane; aktuelne provere i ograničenja su u [statusu projekta](docs/project-status.md). Ubuntu 24.04 x86_64 deb/AppImage i macOS 15.7.9 app/DMG na Apple Silicon/Intel imaju stvarne dokaze izvršavanja. [Konačna native matrica](docs/platform-matrix.md) potvrđuje SSH/Docker/PTY na sve tri klijentske platforme. Nema odobrene javne objave; Mac Developer ID potpis/notarizacija nisu provereni bez vlasnikovih kredencijala. Workflow za javnu objavu je izostavljen po dogovoru.

## 1. Instalacija

Preuzmi odgovarajući **razvojni** paket iz uspešnog [CI izvršavanja vlasničkog repozitorijuma](https://github.com/alekpopovic/container-desk/actions/workflows/ci.yml), za tačan pregledani commit i arhitekturu. Proveri metadata i SHA256SUMS prema [uputstvu za ažuriranje](docs/updates.md). CI artefakti traju 14 dana i ne predstavljaju odobren javni release.

Na Ubuntu 24.04 x86_64, posle provere preuzetih fajlova:

```sh
sha256sum --check SHA256SUMS
sudo apt install ./containerdesk-0.1.0-x86_64-unknown-linux-gnu.deb
```

Pokreni **ContainerDesk** iz menija aplikacija. Detalji i ograničenja AppImage formata su u [Linux paketima](docs/linux-packages.md). Na Mac-u koristi paket za svoj procesor, proveri `shasum -a 256 --check SHA256SUMS`, otvori DMG i kopiraj aplikaciju u Applications. Trenutni nepotpisani/ad-hoc paketi su za lokalnu razvojnu proveru; internet Gatekeeper prihvat nije potvrđen. Ne uklanjaj karantin i ne isključuj zaštitu da bi test izgledao uspešan. Pogledaj [Mac pakete](docs/macos-packages.md) i [opciono potpisivanje](docs/signing.md).

## 2. Pripremi pouzdan SSH pristup

Koristi sopstveni pregledani `~/.ssh/config`, postojeći ključ/agent i servere za koje imaš dozvolu. Sledeći nazivi i adrese su **primeri**, ne stvarni serveri za test. Prilagodi ih informacijama administratora; ne prepisuj postojeći config:

```sshconfig
Host cd-direct
    HostName docker.example
    User operator
    IdentityFile ~/.ssh/id_ed25519
    IdentitiesOnly yes
    StrictHostKeyChecking yes
    ForwardAgent no

Host cd-jump
    HostName bastion.example
    User jumpuser
    IdentityFile ~/.ssh/id_ed25519
    IdentitiesOnly yes
    StrictHostKeyChecking yes
    ForwardAgent no

Host cd-private
    HostName docker.internal.example
    User operator
    IdentityFile ~/.ssh/id_ed25519
    IdentitiesOnly yes
    ProxyJump cd-jump
    StrictHostKeyChecking yes
    ForwardAgent no
```

Za prvo uspostavljanje poverenja pribavi otiske host ključeva nezavisno od administratora. U svom terminalu poveži direktni server, a za privatni put prvo bastion pa odredište; prihvati ključ samo ako se otisak poklapa:

```sh
ssh -o StrictHostKeyChecking=ask cd-direct true
ssh -o StrictHostKeyChecking=ask cd-jump true
ssh -o StrictHostKeyChecking=ask cd-private true
```

Ako se već poznat ključ promenio, prvo razjasni uzrok sa administratorom. Aplikacija ne prihvata nepoznate ključeve, ne menja known_hosts i ne prikuplja lozinke. Config je pouzdana izvršiva konfiguracija: `Match exec`/`ProxyCommand` mogu izvršavati lokalne komande pri eksplicitnom resolve/connect koraku. Samo pretraživanje aliasa čita fajlove.

Proveri agent u sesiji iz koje će aplikacija biti pokrenuta. Ako je potrebno, učitaj **svoj postojeći** ključ u agent u terminalu; lozinku unosiš u `ssh-add`, nikada u aplikaciju:

```sh
ssh-add -l
ssh-add ~/.ssh/id_ed25519
ssh -o BatchMode=yes cd-direct docker version
ssh -o BatchMode=yes cd-private docker info
```

Uspešan pristup bastionu nije dokaz pristupa odredištu. GUI pokrenut iz menija/Finder-a može imati drugi PATH i agent od terminala. Koristi svoj desktop agent ili poznat `IdentityAgent` u SSH config-u; detalji su u [pokretanju sa desktopa](docs/desktop-launch.md). Agent forwarding nije potreban. Cilj je Linux Docker sa POSIX-kompatibilnim neinteraktivnim shell-om; aplikacija ne instalira ništa na server.

## 3. Dodaj server i poveži se

1. Otvori **Settings → Run diagnostics**. Proveri dostupni OpenSSH i agent. „Reachable“ agent znači dostupan socket, ne potvrdu da je odgovarajući ključ učitan. Po potrebi zadaj pouzdan apsolutni **OpenSSH executable override**.
2. Otvori **Hosts → New host** (ili **Add host** u bočnom panelu, pa **New host**). Ostavi **Host SSH config path** prazno za podrazumevani config ili unesi apsolutnu putanju pouzdanog fajla.
3. Izaberi **Browse aliases → Use cd-direct** / **Use cd-private**, ili unesi konkretan **Host SSH alias** ručno. Dozvoljeni su ASCII slova/cifre, tačka, crtica i donja crta, uz prvo slovo/cifru; wildcard i opcije nisu aliasi.
4. Unesi **Display name**. **Saved Docker executable** može ostati prazan za udaljeni neinteraktivni PATH; inače koristi npr. `/usr/bin/docker`. **Saved Docker context** ostaje prazan za efektivni kontekst udaljenog korisnika, ili unesi njegov postojeći naziv. **Use existing sudo -n Docker access** uključi samo za već podešenu neinteraktivnu sudo politiku.
5. Klikni **Save host**, zatim **Connect saved host**. Čuvanje ne uspostavlja vezu. Proveri status **Ready · SSH session**, SSH cilj/jump put i stvarni Docker endpoint/daemon identitet. Rootless kontekst pripada udaljenom korisniku; sudo može odabrati drugi daemon.
6. Otvori **Containers**, izaberi kontejner, pregledaj detalje; **Start logs** prati logove, **Stop logs** prekida praćenje. Statistika i ostali resursi koriste isti potvrđeni host/daemon. Prazan rezultat, zastareo prikaz i greška nisu isto stanje.
7. Kada završiš, **Hosts → Disconnect saved host**. Nova ili obnovljena sesija počinje samo za čitanje.

## 4. Upravljanje, Compose i terminal

**Enable management** je privremena dozvola za izabrani host. Akcija zatim prikazuje identitet hosta/daemon-a, tačne kontejnere i operaciju; **Confirm action** izvršava, **Cancel action** odustaje pre slanja. Uklanjaju se samo eksplicitno izabrani zaustavljeni kontejneri, bez force-a i uklanjanja volumena. Read-only kontrola aplikacije ne smanjuje ovlašćenja SSH/Docker naloga na serveru.

Compose grupisanje ne daje dozvolu za izvršavanje. Za start/stop/restart postojećih servisa moraš navesti i potvrditi udaljeni radni direktorijum, uređenu listu apsolutnih config putanja i tačno ime projekta, pa uraditi **Verify remote project**. Potrebni su čitljivi fajlovi, dostupne interpolacione vrednosti i konfiguracija koja odgovara već pokrenutom projektu. Nema kreiranja/ponovnog deployment-a nedostajućih servisa. [Detalji](docs/compose-actions.md).

U kartici **Terminal** omogući management ako je potrebno, zatim **Enable terminal access → Open terminal** i pregledaj potvrdu korisnika/shell-a/cilja pre **Confirm terminal**. Terminal može menjati stanje; read-only pregled ga ne dozvoljava. Neuspešan ili izgubljen odgovor posle akcije znači mogući **unknown** ishod: osveži stanje i proveri na serveru pre nove odluke. Aplikacija ne ponavlja automatski mutacije niti terminalski unos.

## Pomoć, podaci i razvoj

Najčešći uzroci problema i koraci su u [English user guide](docs/user-guide.md#troubleshooting): nepoznat host ključ, jump autentikacija, GUI PATH/agent, Docker dozvole/kontekst, logging driver, Compose verifikacija i neizvestan ishod akcije.

Podešavanja su u `~/.local/share/dev.containerdesk.app/preferences` na tipičnom Linux-u ili `~/Library/Application Support/dev.containerdesk.app/preferences` na Mac-u; Linux poštuje `XDG_DATA_HOME`. Čuvaju se reference i metapodaci, ne ključevi, lozinke, logovi ili terminalski transkripti. Log export može sadržati tajne iz samih logova: pregledaj sadržaj pre deljenja. **Settings → Prepare support preview → Save reviewed report…** čuva samo pregledani izveštaj u lokaciju koju izabereš, bez automatskog slanja. [Backup i rollback](docs/updates.md), [lokalni podaci](docs/settings.md).

Za razvoj pročitaj [CONTRIBUTING](CONTRIBUTING.md), [development](docs/development.md) i [arhitekturu](codex/docs/ARCHITECTURE.md). Originalnih 60 promptova i dokazi ostaju u `codex/`; oni nisu korisnički installer. Nastavak razvoja prati `python3 codex/scripts/track.py next`. Sintetički **Open demo** ne pristupa SSH-u i nije dokaz native rada.
