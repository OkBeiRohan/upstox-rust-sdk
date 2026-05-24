# Upstox Rust SDK — GitHub Project Setup Guide

The `gh` CLI currently cannot create GitHub Projects v2 custom fields or views. Please follow these manual steps to finish setting up your project board.

## 1. Create the Project (if you haven't already)
If the automated script failed to create the project due to permissions, go to your GitHub user profile -> Projects -> New Project -> Table (or Board) and name it "Upstox Rust SDK — Code Review & Development".

## 2. Add Custom Fields
Go to your Project -> Settings (or click the column header -> New field).

### A. Priority (Single Select)
Create a new "Single select" field named **Priority**. Add these options:
- `P0-critical` (Red)
- `P1-high` (Orange)
- `P2-medium` (Yellow)
- `P3-low` (Green)

### B. Module (Single Select)
Create a new "Single select" field named **Module**. Add these options (you can use one color for all):
- `client`, `rate_limiter`, `ws_client`, `apis/login`, `apis/orders`, `apis/market_quote`, `apis/historical_data`, `apis/instruments`, `apis/portfolio`, `apis/charges`, `apis/margins`, `apis/gtt_orders`, `apis/option_chain`, `apis/market_info`, `apis/trade_pnl`, `apis/user`, `apis/expired_instruments`, `models`, `models/ws`, `models/orders`, `models/user`, `utils`, `constants`, `protos`

### C. Review Status (Single Select)
Create a new "Single select" field named **Review Status**. Add these options:
- `Not Started` (Gray)
- `In Review` (Yellow)
- `Changes Requested` (Red)
- `Approved` (Green)
- `Tests Written` (Blue)

### D. Estimated Hours (Number)
Create a new "Number" field named **Estimated Hours**.

### E. Sprint (Iteration)
Create a new "Iteration" field named **Sprint**. Set duration to 2 weeks.

## 3. Create Views
Click the `+` icon next to the view tabs to create new views.

### View 1: Review Board
- **Layout**: Board
- **Column by**: Review Status
- **Filter**: `label:"type/code-review"`

### View 2: All Issues
- **Layout**: Table
- **Sort by**: Priority (Ascending/Descending as preferred)
- Show fields: Title, Assignees, Status, Priority, Module, Estimated Hours, Sprint

### View 3: By Module
- **Layout**: Table
- **Group by**: Module

### View 4: Sprint Timeline
- **Layout**: Roadmap
- **Date fields**: Iteration (Sprint)

## 4. Update the Workflow
Once your project is created, copy its URL (e.g., `https://github.com/users/OkBeiRohan/projects/X`).
Edit `.github/workflows/add-to-project.yml` and replace the `project-url` with your actual URL.

## 5. Add the Secret
Create a Personal Access Token (Classic) with `project` and `read:org` scopes.
Go to the repository Settings -> Secrets and variables -> Actions.
Add a new repository secret named `PROJECTS_PAT` with the token value.
