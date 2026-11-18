# AGENTS.md — OpenMAD-Rust

Guia técnico de arquitetura, fluxo da aplicação, convenções de desenvolvimento e processo de release do **OpenMAD-Rust** para agentes de IA e contribuidores.

---

## 1. Visão Geral e Repositórios

O **OpenMAD-Rust** é um driver nativo, utilitário de telemetria e configurador GUI 100% em Rust para teclados magnéticos Hall-Effect **MAD68 HE / MAD60 HE** (`VID 0x373B`).

### Remotes Git (Sincronização Obrigatória)
O remote `origin` está configurado com **duas URLs de push simultâneas**:
- **GitHub**: `https://github.com/muriloscolari/OpenMAD-Rust.git` (usado pelo CI/CD e pelo auto-updater do aplicativo)
- **Forgejo**: `https://git.muriloscolari.me/murilo/OpenMAD-Rust.git`
- **Upstream original (referência de protocolo Python/Web)**: `https://github.com/acciaw/OpenMAD.git`

Todo `git push origin main` e `git push origin <tag>` envia automaticamente para GitHub e Forgejo.

---

## 2. Estrutura do Workspace

```text
OpenMAD/
├── Cargo.toml                  # Workspace Cargo (versão em [workspace.package])
├── version.ini                 # Versão e metadados lidos pelo Inno Setup (Windows)
├── .github/workflows/release.yml # Pipeline CI de build Linux + Windows e GitHub Release
├── assets/
│   ├── fonts/Inter-Variable.ttf
│   ├── icon.png / icon.ico     # Ícone squircle gerado proceduralmente
│   └── screenshots/            # Capturas reais em PNG usadas nos READMEs
├── crates/
│   ├── openmad-core/src/
│   │   ├── lib.rs              # Exporta os módulos do core
│   │   ├── protocol.rs         # Codec VIA/HID (pacotes, offsets, conversão 0.00–3.50 mm)
│   │   ├── devices.rs          # Tabela de PIDs suportados (MAD68 HE RGB, MAD60 HE, MAD60 HE RGB) e layouts
│   │   ├── features.rs         # Estruturas de LightInfo, BoxLight, DeadBand, DKS, TapDance, Macros
│   │   ├── keycodes.rs         # Mapas de códigos de tecla padrão, mídia, camada e sistema
│   │   ├── device.rs           # Sessão USB HID (hidapi), leitura de flash (FullConfig) e escrita RAM/Flash
│   │   ├── sampler.rs          # Thread de background: conexão, fila DeviceCommand, telemetria e switcher
│   │   ├── profile.rs          # Serialização JSON de perfis e registro onboard
│   │   ├── backup.rs           # Exportação e restauração completa de configurações
│   │   ├── switcher.rs         # Detecção de janela ativa (KDE Wayland via KWin D-Bus / X11 / Win32)
│   │   ├── settings.rs         # Preferências (idioma, ganho RGB, autostart --minimized)
│   │   ├── updater.rs          # Consulta GitHub Releases e aplica atualização silenciosa
│   │   └── paths.rs            # Diretórios de dados (perfis, settings.json, switcher.json)
│   └── openmad-ui/src/
│       ├── main.rs             # Entrypoint iced::daemon e registro .desktop/.png no Linux
│       ├── app.rs              # Estado OpenMadApp, Update, Subscriptions, Views e --screenshot=<tela>
│       ├── tray.rs             # Bandeja do sistema (tray-icon + thread GTK/libappindicator no Linux)
│       ├── theme.rs            # Paleta Apple Dark Mode, estilos de widgets e fonte Inter
│       ├── i18n.rs             # Traduções dinâmicas PT-BR / EN
│       ├── icon.rs             # Gerador procedural do ícone RGBA 256x256
│       └── widgets/            # KeyboardCanvas, ColorPickerCanvas (HSV 2D) e TravelGaugeBar
└── packaging/
    ├── arch/                   # PKGBUILD e install-arch.sh (.pkg.tar.zst)
    ├── debian/                 # build-deb.sh (.deb via dpkg-deb ou fallback tar + ar)
    ├── linux/                  # 60-openmad.rules (udev) e openmad.desktop
    └── windows/                # openmad.iss (Inno Setup 6)
```

---

## 3. Fluxo Completo da Aplicação

### 3.1 Inicialização (`main.rs` -> `OpenMadApp::new`)
1. No Linux, `install_linux_desktop_entry()` gera/atualiza o ícone PNG (`~/.local/share/icons/hicolor/256x256/apps/openmad-rust.png`) e o atalho `~/.local/share/applications/openmad.desktop` apontando para `std::env::current_exe()`, atualizando caches (`gtk-update-icon-cache` e `kbuildsycoca6`) quando houver mudança.
2. `iced::daemon` inicia com `OpenMadApp::new()`:
   - Lê flags de CLI:
     - `--minimized` ou `--tray`: inicia sem abrir janela (`window_id = None`), mantendo o app ativo apenas na bandeja do sistema.
     - `--screenshot=<nome>`: modo de captura automática de framebuffer via `iced::window::screenshot(id)` (salva `assets/screenshots/<nome>.png` e encerra).
   - Carrega `AppSettings` (`settings.json`), `SwitcherConfig` (`switcher.json`) e lista de perfis (`profiles/`).
   - Inicia a thread do driver com `SamplerHandle::start()`.
   - Inicia `TrayHandle::new(&profiles)` (no Linux, sobe uma thread própria com `gtk::init()` + `tray_icon`).
   - Se `settings.check_updates` for `true`, dispara `check_for_update()` em thread separada contra a API de releases de `muriloscolari/OpenMAD-Rust`.

### 3.2 Comunicação UI <-> Driver (`sampler.rs`)
- A UI não acessa `hidapi` diretamente. A comunicação é desacoplada em dois sentidos:
  - **UI -> Driver**: envio de variantes de `DeviceCommand` por canal MPSC (`crossbeam_channel`).
  - **Driver -> UI**: leitura de `Snapshot` compartilhado (`Arc<RwLock<Snapshot>>`) a cada `33 ms` (`Message::Tick` em `OpenMadApp::subscription()`).
- Em cada ciclo da thread `sampler.rs`:
  1. **Conexão / Reconexão**: procura dispositivo `VID 0x373B` na interface VIA (`usage_page 0xFF60`). Ao conectar, lê `FullConfig` (iluminação, atuação, Rapid Trigger, deadband, keymaps, DKS, TapDance, macros) e verifica `protocol_version` contra `KNOWN_PROTOCOL_VERSION` (ativando `read_only` por segurança se o firmware for desconhecido até desbloqueio explícito).
  2. **Fila de Comandos**: drena e executa comandos pendentes (`SetLight`, `SetActuation`, `SetRapidTrigger`, `SetKeyColors`, `ApplyProfile`, `CommitToFlash`, `FactoryReset`, etc.).
  3. **Telemetria Hall-Effect**: lê o curso analógico das teclas em tempo real (`travel_to_mm`, escala `0.00–3.50 mm`), atualizando `trip_mm` e `peak_mm` no `Snapshot`.
  4. **App Switcher (`switcher.rs`)**:
     - **Linux (KDE Plasma Wayland)**: registra/consulta via D-Bus uma ponte KWin Scripting (`openmad_kwin_bridge`) para obter a classe/executável e título da janela em foco sem XWayland forçado.
     - **Windows**: consulta `GetForegroundWindow()` e o processo dono da janela.
     - Quando a janela ativa casa com uma regra configurada, aplica o perfil correspondente usando `FlashOp::Normal`.

### 3.3 Segurança de Memória Flash vs. Pré-visualização em RAM
- Ajustes interativos (sliders de brilho, cor no seletor 2D HSV, atuação, troca automática de perfil por app) usam **`FlashOp::Normal`** — aplicam ao vivo no teclado sem desgaste contínuo da memória flash interna.
- Gravações permanentes na memória do teclado ocorrem apenas sob ação explícita do usuário (`CommitToFlash` / **Salvar na Flash** / **Aplicar Keymap Completo (EEPROM)**).

### 3.4 Ciclo de Vida da Janela, Bandeja e Autostart
- Por limitações do Wayland (onde `set_visible(false)` em `winit` é no-op), o app usa **`iced::daemon`**:
  - Clicar no `X` (`Message::WindowCloseRequested`) ou no botão **Bandeja** executa `window::close(id)`, destruindo apenas a superfície da janela e mantendo `sampler` + `tray` ativos.
  - Clicar no ícone da bandeja ou em "Abrir" chama `window::open(crate::window_settings())`.
  - O menu de contexto da bandeja (`TrayAction`) permite abrir/ocultar janela, ativar/pausar App Switcher, gravar na flash, trocar perfil ativo ou sair completamente (`sampler.stop()` + `iced::exit()`).
- **Autostart (`settings.rs`)**: `set_start_at_login(enabled)` cria/remove `~/.config/autostart/openmad.desktop` com `--minimized` (Linux) ou a entrada `OpenMAD` com `--minimized` em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (Windows).

### 3.5 Auto-Updater (`updater.rs`)
- `check_for_update()` consulta `https://api.github.com/repos/muriloscolari/OpenMAD-Rust/releases/latest` e compara com `APP_VERSION` (`CARGO_PKG_VERSION`).
- `download_and_apply_update()`:
  - **Windows**: baixa `OpenMAD-Setup-*.exe` e executa com `/SILENT /CLOSEAPPLICATIONS`.
  - **Linux**: baixa `openmad-linux-x86_64` e substitui `std::env::current_exe()` (funciona sem `sudo` quando instalado em `~/.local/bin/openmad`).

---

## 4. Regras e Convenções para Agentes

- **Testes obrigatórios**: rode `cargo test --workspace` após qualquer alteração em `crates/`.
- **Traduzir toda string nova**: use sempre `self.lang.t("Texto PT-BR", "Text EN")` em `openmad-ui` (`crates/openmad-ui/src/i18n.rs`).
- **Captura de screenshots**: devido ao comportamento do `iced_wgpu`/`iced_glyphon` que limpa o atlas de texto após a primeira captura `window::screenshot(id)` no mesmo processo, **sempre capture uma tela por processo** usando:
  ```bash
  cargo build --bin openmad
  for screen in lighting performance app-trigger remap settings; do
    MANGOHUD=0 ./target/debug/openmad --screenshot="$screen"
  done
  ```
- **Commits e sincronização**: sempre verifique `git status` para não commitar `dist/` ou `target/` (já ignorados no `.gitignore`), e faça push para `origin` garantindo atualização no **GitHub** e no **Forgejo**.

---

## 5. Como Fazer uma Nova Release

### Passo 1 — Atualizar Versão nos Manifestos
Atualize o número da versão (ex.: `0.2.0`) nos arquivos:
1. [`Cargo.toml`](Cargo.toml) (`[workspace.package] version = "X.Y.Z"`)
2. [`version.ini`](version.ini) (`version = X.Y.Z` — usado pelo Inno Setup)
3. [`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD) (`pkgver=X.Y.Z`)
4. [`packaging/arch/install-arch.sh`](packaging/arch/install-arch.sh) (`PKGVER="X.Y.Z"`)

Em seguida, valide testes e atualize o `Cargo.lock`:
```bash
cargo test --workspace
```

### Passo 2 — Gerar Pacotes Linux Localmente (opcional / para pacote Arch)
```bash
cargo build --release --bin openmad
mkdir -p dist
cp target/release/openmad dist/openmad-linux-x86_64
bash packaging/debian/build-deb.sh X.Y.Z
bash packaging/arch/install-arch.sh
```
Isso gera em `dist/`:
- `openmad-linux-x86_64`
- `openmad_X.Y.Z_amd64.deb`
- `openmad-rust-X.Y.Z-1-x86_64.pkg.tar.zst`

### Passo 3 — Commit, Tag e Push Dual (GitHub + Forgejo)
```bash
git add -A
git commit -m "chore(release): vX.Y.Z"
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

### Passo 4 — Pipeline Automático (GitHub Actions)
O push da tag `vX.Y.Z` dispara automaticamente [`.github/workflows/release.yml`](.github/workflows/release.yml):
1. **`linux`**: roda `cargo test --workspace`, compila `openmad-linux-x86_64` e cria `openmad_X.Y.Z_amd64.deb`.
2. **`windows`**: roda `cargo test --workspace`, compila `openmad-windows-x86_64.exe` e gera o instalador `OpenMAD-Setup-X.Y.Z.exe` via Inno Setup 6 ([`packaging/windows/openmad.iss`](packaging/windows/openmad.iss)).
3. **`publish`**: cria/atualiza a GitHub Release `vX.Y.Z` com os artefatos acima.

Acompanhe o workflow pelo terminal:
```bash
gh run list -R muriloscolari/OpenMAD-Rust --limit 3
gh run watch -R muriloscolari/OpenMAD-Rust
```

### Passo 5 — Anexar Pacote Arch no GitHub e Espelhar Release no Forgejo
Após o CI finalizar, baixe os artefatos Windows (se quiser espelhar no Forgejo) e envie também o `.pkg.tar.zst` para o GitHub:
```bash
# Anexar o pacote Arch Linux à release do GitHub
gh release upload vX.Y.Z -R muriloscolari/OpenMAD-Rust \
  dist/openmad-rust-X.Y.Z-1-x86_64.pkg.tar.zst --clobber

# Baixar artefatos Windows gerados pelo CI
gh run download --name windows-artifacts -R muriloscolari/OpenMAD-Rust -D dist/

# Criar release espelhada no Forgejo
gt release create vX.Y.Z \
  dist/OpenMAD-Setup-X.Y.Z.exe \
  dist/openmad-windows-x86_64.exe \
  dist/openmad-linux-x86_64 \
  dist/openmad_X.Y.Z_amd64.deb \
  dist/openmad-rust-X.Y.Z-1-x86_64.pkg.tar.zst \
  -R https://git.muriloscolari.me/murilo/OpenMAD-Rust \
  --title "OpenMAD-Rust vX.Y.Z" \
  --verify-tag
```
