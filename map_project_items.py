import subprocess
import json
import time

def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout.strip()

# 1. Add Milestone to all issues
print("Adding milestone to all issues...")
issues_json = run(['gh', 'issue', 'list', '--repo', 'OkBeiRohan/upstox-rust-sdk', '--limit', '50', '--json', 'number,labels'])
issues = json.loads(issues_json)

for issue in issues:
    num = issue['number']
    subprocess.run(['gh', 'issue', 'edit', str(num), '--repo', 'OkBeiRohan/upstox-rust-sdk', '--milestone', 'V2 Codebase Review'])

# 2. Get Project Node ID
print("Fetching Project Node ID...")
projects_json = run(['gh', 'project', 'list', '--owner', 'OkBeiRohan', '--format', 'json'])
projects = json.loads(projects_json)
project_id = None
for p in projects['projects']:
    if p['number'] == 6:
        project_id = p['id']
        break

if not project_id:
    print("Project 6 not found!")
    exit(1)

# 3. Get Project Fields
print("Fetching Project Fields...")
fields_json = run(['gh', 'project', 'field-list', '6', '--owner', 'OkBeiRohan', '--format', 'json'])
fields = json.loads(fields_json)

fields_map = {}
for f in fields['fields']:
    name = f['name']
    fields_map[name] = f
    
def get_option_id(field_name, option_name):
    field = fields_map.get(field_name)
    if not field or 'options' not in field: return None
    for opt in field['options']:
        if opt['name'].lower() == option_name.lower():
            return opt['id']
    return None

# 4. Get Project Items
print("Fetching Project Items...")
items_json = run(['gh', 'project', 'item-list', '6', '--owner', 'OkBeiRohan', '--format', 'json'])
items = json.loads(items_json)

print("Updating Project Items...")
for item in items['items']:
    item_id = item['id']
    content = item.get('content', {})
    issue_num = content.get('number')
    if not issue_num: continue
    
    # Find matching issue to get labels
    issue = next((i for i in issues if i['number'] == issue_num), None)
    if not issue: continue
    
    labels = [l['name'] for l in issue['labels']]
    
    # Extract priority and area
    priority = next((l.replace('priority/', '') for l in labels if l.startswith('priority/')), None)
    area = next((l.replace('area/', '') for l in labels if l.startswith('area/')), None)
    
    commands = []
    
    # Priority Field
    if priority and 'Priority' in fields_map:
        opt_id = get_option_id('Priority', priority)
        if opt_id:
            commands.append(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Priority']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])
            
    # Module Field
    if area and 'Module' in fields_map:
        opt_id = get_option_id('Module', area)
        if opt_id:
            commands.append(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Module']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])
            
    # Review Status Field
    if 'Review Status' in fields_map:
        opt_id = get_option_id('Review Status', 'Not Started')
        if opt_id:
            commands.append(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Review Status']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])
            
    # Estimated Hours Field
    if 'Estimated Hours' in fields_map:
        # Epic gets 20 hours, sub-issues get 2 hours
        hours = "20" if issue_num == 1 else "2"
        commands.append(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Estimated Hours']['id'], '--project-id', project_id, '--number', hours])
        
    # Default Status Field
    if 'Status' in fields_map:
        opt_id = get_option_id('Status', 'Todo')
        if opt_id:
            commands.append(['gh', 'project', 'item-edit', '--id', item_id, '--field-id', fields_map['Status']['id'], '--project-id', project_id, '--single-select-option-id', opt_id])

    for cmd in commands:
        subprocess.run(cmd)

print("All items successfully mapped and updated!")
