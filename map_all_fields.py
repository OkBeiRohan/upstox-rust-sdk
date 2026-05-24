import subprocess
import json
import time

def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout.strip()

# Get Project Fields
fields_json = run(['gh', 'project', 'field-list', '6', '--owner', 'OkBeiRohan', '--format', 'json'])
fields = json.loads(fields_json)

fields_map = {f['name']: f for f in fields['fields']}
project_id = 'PVT_kwHOAhswOc4BYok6'

def get_option_id(field_name, option_name):
    field = fields_map.get(field_name)
    if not field or 'options' not in field: return None
    for opt in field['options']:
        if opt['name'].lower() == option_name.lower() or opt['name'].lower().startswith(option_name.lower()):
            return opt['id']
    return None

# Get Project Items
items_json = run(['gh', 'project', 'item-list', '6', '--owner', 'OkBeiRohan', '--format', 'json'])
items = json.loads(items_json)['items']

# Get Issues
issues_json = run(['gh', 'issue', 'list', '--repo', 'OkBeiRohan/upstox-rust-sdk', '--limit', '50', '--json', 'number,labels'])
issues = json.loads(issues_json)

for item in items:
    item_id = item['id']
    issue_num = item.get('content', {}).get('number')
    if not issue_num: continue
    
    issue = next((i for i in issues if i['number'] == issue_num), None)
    if not issue: continue
    
    labels = [l['name'] for l in issue['labels']]
    
    # Priority mapping (P0-critical -> P0)
    priority_label = next((l.replace('priority/', '') for l in labels if l.startswith('priority/')), None)
    if priority_label:
        p_val = priority_label.split('-')[0] # 'P0'
        opt_id = get_option_id('Priority', p_val)
        if not opt_id and p_val == 'P3': 
            opt_id = get_option_id('Priority', 'P2') # map P3 to P2 if P3 doesn't exist
        if opt_id:
            subprocess.run(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Priority']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])

    # Status -> To triage
    opt_id = get_option_id('Status', 'To triage')
    if opt_id:
        subprocess.run(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Status']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])

    # Module
    area = next((l.replace('area/', '') for l in labels if l.startswith('area/')), None)
    if area:
        opt_id = get_option_id('Module', area)
        if opt_id:
            subprocess.run(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Module']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])

    # Review Status
    opt_id = get_option_id('Review Status', 'Not Started')
    if opt_id:
        subprocess.run(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Review Status']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])

    # Estimate (hours)
    hours = "20" if issue_num == 1 else "2"
    if 'Estimate' in fields_map:
        subprocess.run(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Estimate']['id'], '--project-id', project_id, '--number', hours])

    # Start date
    if 'Start date' in fields_map:
        subprocess.run(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Start date']['id'], '--project-id', project_id, '--date', '2026-05-24'])

    # Target date
    if 'Target date' in fields_map:
        subprocess.run(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Target date']['id'], '--project-id', project_id, '--date', '2026-06-29'])

print("All mapping finished.")
