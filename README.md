# TicketDash

A lightweight, fast desktop dashboard for Jira tickets built with Tauri, React, and TypeScript. Get instant visibility into your assigned tickets, track resolution metrics, and monitor SLA compliance—all without the browser overhead.

## What is TicketDash?

TicketDash is a native desktop application that syncs with Jira to provide a focused, performance-optimized view of your assigned tickets. It pulls your tickets locally, calculates business-hours metrics, and visualizes trends through interactive charts—all while keeping your data synced in the background.

## Why Use TicketDash?

### Speed & Performance
- **Native desktop app** - No browser tabs, no web app lag
- **Local-first data** - Instant searches and filters, even offline
- **Background sync** - Set it and forget it; your data stays fresh automatically

### Better Visibility
- **Visual dashboards** - See ticket distribution by status, priority, and category at a glance
- **Timeline tracking** - Understand ticket creation vs. resolution trends over time
- **Resolution metrics** - Track average and median resolution times by priority level
- **Business hours calculation** - Accurate SLA metrics that respect working hours (9-5 by default)

### Focused Workflow
- **Only your tickets** - No noise from the entire project backlog
- **Customizable filters** - Quickly slice by status, priority, assignee, or category
- **Auto-categorization** - Define rules to organize tickets into meaningful groups
- **Quick search** - Find tickets by key, summary, or description instantly

## What Would You Use It For?

### Daily Standups
- Quick count of in-progress vs. open tickets
- Identify blockers by filtering high-priority items
- Export/print dashboard for team visibility

### SLA Compliance
- Monitor average resolution times by priority
- Track which ticket types take longest to resolve
- Identify trends before they become SLA violations

### Personal Productivity
- Stay focused on assigned work without Jira's full interface
- Reduce context switching between browser tabs
- See your workload distribution at a glance

### Team Leads & Managers
- Aggregate view of team ticket health
- Spot bottlenecks in resolution times
- Track ticket creation vs. completion rates

## How to Use It

### Prerequisites
- **Jira Cloud account** with API access
- **Jira API token** ([create one here](https://id.atlassian.com/manage-profile/security/api-tokens))
- Your Jira instance URL (e.g., `https://yourcompany.atlassian.net`)

### Installation

#### Option 1: Download Release (Recommended)
1. Go to [Releases](https://github.com/samueladad75-byte/TicketDash/releases)
2. Download the installer for your platform:
   - **macOS**: `.dmg` file
   - **Windows**: `.msi` installer
   - **Linux**: `.AppImage` or `.deb`
3. Run the installer and follow prompts

#### Option 2: Build from Source
```bash
# Clone the repository
git clone https://github.com/samueladad75-byte/TicketDash.git
cd TicketDash

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Initial Setup

1. **Launch TicketDash**
2. **Navigate to Settings** (gear icon in sidebar)
3. **Configure Jira connection:**
   - **Jira URL**: Your instance URL (e.g., `https://yourcompany.atlassian.net`)
   - **Email**: Your Jira account email
   - **API Token**: Paste your API token
   - **Sync Interval**: How often to auto-sync (in minutes, 0 to disable)
4. **Save Settings**
5. **Trigger first sync** - Click "Sync Now" in the dashboard

### Features

#### Dashboard View
- **Summary Cards**: Total tickets, open, in progress, resolved counts
- **Status Distribution**: Pie chart showing ticket breakdown by status
- **Priority Distribution**: Visualize high/medium/low priority tickets
- **Category Distribution**: See how tickets are categorized
- **Timeline Chart**: Track ticket creation and resolution trends over time
- **Resolution Time by Priority**: Average and median resolution hours per priority level

#### Tickets View
- **Searchable table** with all your assigned tickets
- **Filters** by status, priority, assignee, category
- **Sortable columns**: Key, summary, status, priority, assignee, created date
- **Click-to-copy** ticket keys for quick Jira lookups

#### Settings View
- **Jira configuration** (URL, email, token)
- **Sync settings** (auto-sync interval)
- **Business hours** (for SLA calculations, default 9 AM - 5 PM)

### Background Sync

Once configured, TicketDash automatically syncs your tickets in the background:
- Runs at your configured interval (e.g., every 15 minutes)
- Only fetches tickets updated since last sync (efficient)
- Shows progress bar during sync
- Updates all charts and metrics automatically

You can also manually trigger a sync anytime by clicking "Sync Now."

## Tech Stack

- **Tauri** - Rust-powered native desktop framework
- **React 18** - UI framework with functional components and hooks
- **TypeScript** - Type-safe frontend development
- **Zustand** - Lightweight state management
- **Recharts** - Interactive charts and visualizations
- **SQLite** - Local database for ticket storage
- **Tauri Store** - Encrypted settings storage

## Development

### Project Structure
```
TicketDash/
├── src/                    # React frontend
│   ├── components/         # UI components (Dashboard, Tickets, Settings)
│   ├── stores/            # Zustand state management
│   ├── hooks/             # Custom React hooks
│   └── types/             # TypeScript type definitions
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── commands/      # Tauri commands (sync, settings, tickets)
│   │   ├── db/            # SQLite queries and migrations
│   │   ├── jira/          # Jira API client
│   │   ├── services/      # Business logic (categorizer, scheduler, time calc)
│   │   └── models/        # Data models
│   └── Cargo.toml         # Rust dependencies
└── package.json           # Node dependencies
```

### Available Scripts

```bash
# Development
npm run dev              # Run Vite dev server only
npm run tauri dev        # Run full Tauri app in dev mode
npm run lean:dev         # Run Tauri in low-disk lean mode (ephemeral caches)

# Building
npm run build            # Build frontend
npm run tauri build      # Build production app

# Testing
npm run test             # Run frontend tests once
npm run lint             # Lint frontend code

# Cleanup
npm run clean:heavy      # Remove heavy build artifacts only (keeps dependencies)
npm run clean:full       # Remove all reproducible local caches (includes node_modules)
```

### Normal Dev vs Lean Dev

- **Normal dev (`npm run tauri dev`)**: fastest repeated startup because Rust/Vite build caches stay in the repo (`src-tauri/target`, `node_modules/.vite`).
- **Lean dev (`npm run lean:dev`)**: uses temporary cache locations for Rust and Vite, and auto-cleans heavy artifacts when you exit. This keeps project disk usage low but increases startup/compile time.

Use lean mode when disk pressure matters more than startup speed, and normal mode when you are actively iterating and want fastest rebuilds.

### Cleanup Commands

- `npm run clean:heavy`: safe day-to-day cleanup for generated build output and heavy caches only.
- `npm run clean:full`: deeper cleanup that also removes reproducible dependency caches (for example `node_modules`), so the next run needs reinstall/rebuild.

### Code Standards
- **Rust**: No `unwrap()` in production code, proper error handling with `thiserror`
- **React**: Functional components only, hooks, no class components
- **TypeScript**: Strict mode enabled, no `any` types
- **Commits**: Conventional commits (feat/fix/docs/refactor/test)

## Security

- **API tokens** are stored securely using Tauri's encrypted store plugin
- **Local data** is stored in SQLite database in your app data directory
- **No telemetry** - Your data stays on your machine
- **SQL injection protection** - All queries use parameterized statements

## Roadmap

- [ ] **Export to CSV/PDF** - Export ticket lists and dashboards
- [ ] **Category rule UI** - Configure auto-categorization rules in settings
- [ ] **Custom date ranges** - Filter timeline charts by date
- [ ] **Multi-account support** - Switch between multiple Jira instances
- [ ] **Advanced filters** - Filter by labels, components, sprints
- [ ] **Notifications** - Desktop alerts for ticket updates
- [ ] **Dark mode** - Theme customization

## Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch (`git checkout -b feat/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) for details

## Support

- **Issues**: [GitHub Issues](https://github.com/samueladad75-byte/TicketDash/issues)
- **Discussions**: [GitHub Discussions](https://github.com/samueladad75-byte/TicketDash/discussions)

---

Built with ❤️ using Tauri, React, and TypeScript
