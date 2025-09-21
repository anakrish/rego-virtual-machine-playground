// RVM Playground Application

class RVMPlayground {
    constructor() {
        this.editors = {};
        this.wasmModule = null;
        this.currentProgram = null;
        this.settings = {
            theme: 'vs',
            wordWrap: true,
            minimap: true,
            showAddresses: true,
            showComments: true,
            autoEvaluate: false,
            showTiming: true
        };
        
        this.entryPoints = ['data.authz.allow'];
        
        this.examples = {
            categories: {
                'Basic': [
                    {
                        name: "Simple Conditions",
                        description: "Basic conditional logic",
                        entryPoints: ["data.basic.conditions.result"],
                        policy: `package basic.conditions

import rego.v1

default result := {}

result := {
    "is_admin": is_admin,
    "is_active": is_active,
    "access_level": access_level
}

is_admin if input.user.role == "admin"
is_active if input.user.status == "active"

access_level := "high" if is_admin
access_level := "medium" if {
    not is_admin
    is_active
}
access_level := "low" if {
    not is_admin
    not is_active
}`,
                        input: '{"user": {"role": "admin", "status": "active"}}',
                        data: '{}'
                    },
                    {
                        name: "Array Processing",
                        description: "Working with arrays and comprehensions",
                        entryPoints: ["data.arrays.even_numbers", "data.arrays.total", "data.arrays.all_positive"],
                        policy: `package arrays

import rego.v1

# Filter even numbers
even_numbers := [x | x := input.numbers[_]; x % 2 == 0]

# Sum all numbers
total := sum(input.numbers)

# Check if all numbers are positive
all_positive if {
    count([x | x := input.numbers[_]; x > 0]) == count(input.numbers)
}

# Find maximum value
max_value := max(input.numbers)`,
                        input: '{"numbers": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}',
                        data: '{}'
                    }
                ],
                'Security': [
                    {
                        name: "RBAC Authorization",
                        description: "Role-based access control with permissions",
                        entryPoints: ["data.rbac.authz.allow"],
                        policy: `package rbac.authz

import rego.v1

default allow := false

# Allow if user has required role for the resource
allow if {
    user_has_permission(input.user.name, input.action, input.resource)
}

# Check if user has permission for action on resource
user_has_permission(user, action, resource) if {
    some role in input.user_roles[user]
    some permission in input.role_permissions[role]
    permission.action == action
    glob.match(permission.resource, [], resource)
}`,
                        input: `{
    "user": {"name": "alice", "id": "123"},
    "action": "read",
    "resource": "/api/users/123",
    "user_roles": {
        "alice": ["admin", "user"],
        "bob": ["user"],
        "charlie": ["viewer"]
    },
    "role_permissions": {
        "admin": [
            {"action": "read", "resource": "/api/*"},
            {"action": "write", "resource": "/api/*"},
            {"action": "delete", "resource": "/api/*"}
        ],
        "user": [
            {"action": "read", "resource": "/api/users/*"},
            {"action": "write", "resource": "/api/users/*"}
        ],
        "viewer": [
            {"action": "read", "resource": "/api/users/*"}
        ]
    }
}`,
                        data: '{}'
                    },
                    {
                        name: "Multi-Factor Authentication",
                        description: "MFA policy with time and device restrictions",
                        entryPoints: ["data.mfa.required", "data.mfa.allow"],
                        policy: `package mfa

import rego.v1

default required := false
default allow := false

# MFA required for admin actions
required if {
    input.action in ["delete", "admin", "configure"]
}

# MFA required for sensitive resources
required if {
    startswith(input.resource, "/admin/")
}

# MFA required during off-hours
required if {
    current_hour := time.clock([time.now_ns(), "UTC"])[0]
    current_hour < 8
}
required if {
    current_hour := time.clock([time.now_ns(), "UTC"])[0]
    current_hour > 18
}

# Allow if MFA not required OR MFA verified
allow if not required
allow if {
    required
    input.mfa_verified == true
    device_trusted
}

device_trusted if {
    input.device_id in input.trusted_devices[input.user]
}`,
                        input: `{
    "user": "admin",
    "action": "delete",
    "resource": "/api/users/123",
    "mfa_verified": true,
    "device_id": "device_456",
    "trusted_devices": {
        "admin": ["device_123", "device_456"],
        "alice": ["device_789"]
    }
}`,
                        data: '{}'
                    }
                ],
                'Kubernetes': [
                    {
                        name: "Pod Security Standards",
                        description: "Kubernetes admission control for pod security",
                        entryPoints: ["data.kubernetes.admission.allow", "data.kubernetes.admission.violations"],
                        policy: `package kubernetes.admission

import rego.v1

default allow := false
default violations := []

# Allow if no violations
allow if count(violations) == 0

# Check for security violations
violations contains msg if {
    input.request.object.spec.containers[_].securityContext.privileged == true
    msg := "Privileged containers are not allowed"
}

violations contains msg if {
    input.request.object.spec.containers[_].securityContext.runAsRoot == true
    msg := "Containers cannot run as root"
}

violations contains msg if {
    container := input.request.object.spec.containers[_]
    not container.resources.limits.memory
    msg := sprintf("Container %s must specify memory limits", [container.name])
}

violations contains msg if {
    container := input.request.object.spec.containers[_]
    not container.resources.limits.cpu
    msg := sprintf("Container %s must specify CPU limits", [container.name])
}`,
                        input: `{
    "request": {
        "object": {
            "spec": {
                "containers": [
                    {
                        "name": "app",
                        "image": "nginx:latest",
                        "securityContext": {
                            "privileged": false,
                            "runAsRoot": false
                        },
                        "resources": {
                            "limits": {
                                "memory": "128Mi",
                                "cpu": "100m"
                            }
                        }
                    }
                ]
            }
        }
    }
}`,
                        data: '{}'
                    },
                    {
                        name: "Network Policies",
                        description: "Kubernetes network security validation",
                        entryPoints: ["data.k8s.network.allow_ingress", "data.k8s.network.allow_egress"],
                        policy: `package k8s.network

import rego.v1

default allow_ingress := false
default allow_egress := false

# Allow ingress from same namespace
allow_ingress if {
    input.source.namespace == input.destination.namespace
}

# Allow ingress from specific allowed namespaces
allow_ingress if {
    input.source.namespace in input.allowed_namespaces[input.destination.namespace]
}

# Allow egress to public internet for specific ports
allow_egress if {
    input.destination.external == true
    input.port in [80, 443, 53]
}

# Allow egress to same cluster
allow_egress if {
    input.destination.external == false
    input.destination.cluster == input.source.cluster
}`,
                        input: `{
    "source": {
        "namespace": "frontend",
        "cluster": "prod-cluster"
    },
    "destination": {
        "namespace": "backend",
        "cluster": "prod-cluster",
        "external": false
    },
    "port": 8080,
    "allowed_namespaces": {
        "backend": ["frontend", "api-gateway"],
        "database": ["backend"]
    }
}`,
                        data: '{}'
                    }
                ],
                'Cloud': [
                    {
                        name: "AWS IAM Policy",
                        description: "Complex AWS IAM authorization",
                        entryPoints: ["data.aws.iam.allow"],
                        policy: `package aws.iam

import rego.v1

default allow := false

# Allow if all conditions are met
allow if {
    user_authenticated
    action_permitted
    resource_accessible
    conditions_met
}

# Check if user is authenticated
user_authenticated if {
    input.context.user.arn
    input.context.user.arn != ""
}

# Check if action is permitted
action_permitted if {
    some policy in input.policies[input.context.user.name]
    some statement in policy.Statement
    statement.Effect == "Allow"
    action_matches(statement.Action, input.request.action)
    resource_matches(statement.Resource, input.request.resource)
}

# Helper functions
action_matches(policy_action, request_action) if policy_action == "*"
action_matches(policy_action, request_action) if policy_action == request_action
action_matches(policy_action, request_action) if {
    endswith(policy_action, "*")
    startswith(request_action, trim_suffix(policy_action, "*"))
}

resource_matches(policy_resource, request_resource) if policy_resource == "*"
resource_matches(policy_resource, request_resource) if policy_resource == request_resource

resource_accessible if {
    region := extract_region(input.request.resource)
    region in input.allowed_regions
}

extract_region(arn) := region if {
    parts := split(arn, ":")
    count(parts) >= 4
    region := parts[3]
}

conditions_met if {
    current_hour := time.clock([time.now_ns(), "UTC"])[0]
    current_hour >= 9
    current_hour <= 17
}`,
                        input: `{
    "context": {
        "user": {
            "name": "alice",
            "arn": "arn:aws:iam::123456789012:user/alice"
        }
    },
    "request": {
        "action": "s3:GetObject",
        "resource": "arn:aws:s3:::my-bucket/data/file.txt"
    },
    "policies": {
        "alice": [
            {
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Action": "s3:*",
                        "Resource": "arn:aws:s3:::my-bucket/*"
                    }
                ]
            }
        ]
    },
    "allowed_regions": ["us-east-1", "us-west-2"]
}`,
                        data: '{}'
                    }
                ],
                'Business Logic': [
                    {
                        name: "Dynamic Pricing",
                        description: "Complex pricing rules with multiple discounts",
                        entryPoints: ["data.business.pricing.discount", "data.business.pricing.final_price"],
                        policy: `package business.pricing

import rego.v1

default discount := 0
default final_price := 0

# Calculate discount based on multiple factors
discount := calculated_discount if {
    calculated_discount := volume_discount + loyalty_discount + seasonal_discount
    calculated_discount <= 50  # Cap at 50%
}

discount := 50 if {
    calculated_discount := volume_discount + loyalty_discount + seasonal_discount
    calculated_discount > 50
}

# Volume-based discount
volume_discount := 0 if input.order.quantity < 10
volume_discount := 5 if {
    input.order.quantity >= 10
    input.order.quantity < 50
}
volume_discount := 10 if {
    input.order.quantity >= 50
    input.order.quantity < 100
}
volume_discount := 15 if input.order.quantity >= 100

# Loyalty discount
loyalty_discount := 0 if not input.customers[input.customer.id]
loyalty_discount := discount_rate if {
    customer := input.customers[input.customer.id]
    years := customer.years_active
    discount_rate := min(years * 2, 20)  # 2% per year, max 20%
}

# Seasonal discount
seasonal_discount := 10 if {
    current_month := time.clock([time.now_ns(), "UTC"])[1]
    current_month in [11, 12]  # November, December
}
seasonal_discount := 0 if {
    current_month := time.clock([time.now_ns(), "UTC"])[1]
    not current_month in [11, 12]
}

# Calculate final price
final_price := price_after_discount if {
    base_price := input.order.unit_price * input.order.quantity
    discount_amount := (base_price * discount) / 100
    price_after_discount := base_price - discount_amount
}`,
                        input: `{
    "customer": {"id": "cust_123"},
    "order": {
        "unit_price": 100,
        "quantity": 25,
        "product_id": "prod_456"
    },
    "customers": {
        "cust_123": {
            "name": "Acme Corp",
            "years_active": 5,
            "tier": "gold"
        }
    }
}`,
                        data: '{}'
                    },
                    {
                        name: "Approval Workflow",
                        description: "Multi-level approval system",
                        entryPoints: ["data.workflow.approval.required_approvers", "data.workflow.approval.can_approve"],
                        policy: `package workflow.approval

import rego.v1

default required_approvers := []
default can_approve := false

# Determine required approvers based on request value
required_approvers := ["manager"] if input.amount <= 1000
required_approvers := ["manager", "director"] if {
    input.amount > 1000
    input.amount <= 10000
}
required_approvers := ["manager", "director", "vp"] if input.amount > 10000

# Check if current user can approve this request
can_approve if {
    user_role := input.users[input.current_user].role
    user_role in required_approvers
    user_in_approval_chain
}

user_in_approval_chain if {
    user_dept := input.users[input.current_user].department
    request_dept := input.users[input.requester].department
    user_dept == request_dept
}

user_in_approval_chain if {
    input.users[input.current_user].role == "vp"  # VPs can approve across departments
}`,
                        input: `{
    "requester": "alice",
    "current_user": "manager_bob",
    "amount": 5000,
    "description": "New laptop purchase",
    "users": {
        "alice": {"role": "employee", "department": "engineering"},
        "manager_bob": {"role": "manager", "department": "engineering"},
        "director_charlie": {"role": "director", "department": "engineering"},
        "vp_diana": {"role": "vp", "department": "executive"}
    }
}`,
                        data: '{}'
                    }
                ],
                'Benchmarking': [
                    {
                        name: "Large Dataset Processing",
                        description: "Process large datasets with complex filtering",
                        entryPoints: ["data.benchmark.large_data.result"],
                        policy: `package benchmark.large_data

import rego.v1

default result := {}

# Process large dataset with complex filtering
result := {
    "high_value_users": high_value_users,
    "active_regions": active_regions,
    "risk_score": overall_risk_score
}

# Find users with high transaction volumes
high_value_users contains user if {
    some user, transactions in input.user_transactions
    total_volume := sum([t.amount | t := transactions[_]])
    total_volume > 10000
    count(transactions) > 3  # Lowered threshold for demo
}

# Identify active regions
active_regions contains region if {
    some region, users in input.region_users
    count(users) > 2  # Lowered threshold for demo
    avg_activity := sum([input.user_activity[u] | u := users[_]]) / count(users)
    avg_activity > 100
}

# Calculate overall risk score
overall_risk_score := score if {
    suspicious_users := {u | 
        some u, transactions in input.user_transactions
        some transaction in transactions
        transaction.amount > 5000
        transaction.country != input.user_profiles[u].home_country
    }
    
    failed_logins := sum([count(input.security_events[u].failed_logins) | u := input.security_events[_]; input.security_events[u]])
    
    score := (count(suspicious_users) * 10) + failed_logins
}`,
                        input: `{
    "user_transactions": {
        "user1": [
            {"amount": 1000, "country": "US"}, {"amount": 2000, "country": "US"},
            {"amount": 3000, "country": "US"}, {"amount": 1500, "country": "US"}
        ],
        "user2": [
            {"amount": 15000, "country": "UK"}, {"amount": 3000, "country": "FR"},
            {"amount": 8000, "country": "DE"}, {"amount": 12000, "country": "ES"}
        ]
    },
    "user_profiles": {
        "user1": {"home_country": "US", "risk_level": "low"},
        "user2": {"home_country": "US", "risk_level": "medium"}
    },
    "region_users": {
        "us-east": ["user1", "user2", "user3"],
        "us-west": ["user4", "user5"]
    },
    "user_activity": {
        "user1": 150, "user2": 200, "user3": 75, "user4": 120, "user5": 180
    },
    "security_events": {
        "user1": {"failed_logins": [1, 2]},
        "user2": {"failed_logins": [1, 2, 3, 4, 5]}
    }
}`,
                        data: '{}'
                    },
                    {
                        name: "Deep Recursion",
                        description: "Recursive organization hierarchy analysis",
                        entryPoints: ["data.benchmark.recursion.result"],
                        policy: `package benchmark.recursion

import rego.v1

default result := {}

# Complex recursive organization hierarchy analysis
result := {
    "total_employees": total_employees,
    "management_levels": management_levels,
    "department_budgets": department_budgets
}

# Count all employees recursively
total_employees := count(all_employees)

all_employees contains emp if {
    some emp in input.employees
}

all_employees contains emp if {
    some dept, members in input.departments
    some emp in members
}

all_employees contains emp if {
    some team, info in input.teams
    some emp in info.members
}

# Calculate management levels
management_levels := max_level if {
    levels := {level | some emp in input.employees; level := employee_level(emp, 0)}
    max_level := max(levels)
}

employee_level(emp, current_level) := level if {
    manager := input.employee_manager[emp]
    level := employee_level(manager, current_level + 1)
}

employee_level(emp, current_level) := current_level if {
    not input.employee_manager[emp]
}

# Calculate department budgets recursively
department_budgets[dept] := budget if {
    some dept, info in input.departments
    employee_costs := sum([input.employee_salary[emp] | emp := info.members[_]])
    subdept_costs := sum([department_budgets[sub] | sub := info.sub_departments[_]])
    budget := employee_costs + subdept_costs
}`,
                        input: `{
    "employees": ["emp1", "emp2", "emp3", "emp4", "emp5"],
    "departments": {
        "engineering": {
            "members": ["emp1", "emp2"],
            "sub_departments": ["backend", "frontend"]
        },
        "backend": {
            "members": ["emp3"],
            "sub_departments": []
        },
        "frontend": {
            "members": ["emp4"],
            "sub_departments": []
        }
    },
    "teams": {
        "security": {"members": ["emp5"]},
        "devops": {"members": ["emp1", "emp3"]}
    },
    "employee_manager": {
        "emp2": "emp1",
        "emp3": "emp1",
        "emp4": "emp1",
        "emp5": "emp1"
    },
    "employee_salary": {
        "emp1": 150000, "emp2": 120000, "emp3": 110000, "emp4": 105000, "emp5": 130000
    }
}`,
                        data: '{}'
                    }
                ],
                'Compilation': [
                    {
                        name: "Advanced Pattern Matching",
                        description: "Showcase various Rego compilation patterns",
                        entryPoints: ["data.compilation.patterns.analysis"],
                        policy: `package compilation.patterns

import rego.v1

default analysis := {}

# Demonstrate various Rego compilation patterns
analysis := {
    "array_patterns": array_analysis,
    "object_patterns": object_analysis,
    "string_patterns": string_analysis,
    "comprehensions": comprehension_analysis
}

# Array pattern matching and manipulation
array_analysis := {
    "filtered": [x | x := input.numbers[_]; x > 10],
    "mapped": [x * 2 | x := input.numbers[_]],
    "reduced": sum(input.numbers),
    "grouped": {
        "even": [x | x := input.numbers[_]; x % 2 == 0],
        "odd": [x | x := input.numbers[_]; x % 2 == 1]
    }
}

# Object pattern matching
object_analysis := {
    "nested_access": input.config.server.host,
    "conditional_fields": {k: v | 
        some k, v in input.metadata
        is_string(v)
        count(v) > 5
    },
    "transformed": {k: upper(v) | 
        some k, v in input.labels
        is_string(v)
    }
}

# String pattern matching and regex
string_analysis := {
    "email_domains": {domain |
        some user in input.users
        regex.match(\`^[^@]+@(.+)$\`, user.email)
        domain := regex.find_n(\`^[^@]+@(.+)$\`, user.email, 1)[0][1]
    },
    "valid_usernames": [user.name |
        some user in input.users
        regex.match(\`^[a-zA-Z][a-zA-Z0-9_]{2,19}$\`, user.name)
    ]
}

# Complex comprehensions
comprehension_analysis := {
    "nested_loop": [
        {"user": user.name, "permission": perm, "resource": res} |
        some user in input.users
        some perm in user.permissions
        some res in input.resources[perm]
    ],
    "conditional_aggregation": {
        dept: {
            "count": count(users),
            "avg_salary": sum([u.salary | u := users[_]]) / count(users)
        } |
        some dept, users in grouped_users
        count(users) > 0
    }
}

# Helper rule for grouping
grouped_users[dept] := users if {
    some dept
    users := [u | some u in input.users; u.department == dept]
}`,
                        input: `{
    "numbers": [1, 5, 10, 15, 20, 25],
    "config": {
        "server": {"host": "localhost", "port": 8080}
    },
    "metadata": {
        "version": "1.0.0",
        "name": "test-app",
        "description": "A comprehensive test application"
    },
    "labels": {
        "env": "production",
        "team": "platform"
    },
    "users": [
        {"name": "alice123", "email": "alice@company.com", "department": "engineering", "salary": 120000, "permissions": ["read", "write"]},
        {"name": "bob_dev", "email": "bob@company.com", "department": "engineering", "salary": 115000, "permissions": ["read"]},
        {"name": "charlie", "email": "charlie@company.com", "department": "sales", "salary": 95000, "permissions": ["read", "admin"]}
    ],
    "resources": {
        "read": ["/api/users", "/api/data"],
        "write": ["/api/users"],
        "admin": ["/api/admin", "/api/config"]
    }
}`,
                        data: '{}'
                    },
                    {
                        name: "Complex Rule Dependencies",
                        description: "Demonstrate complex rule interdependencies and optimization",
                        entryPoints: ["data.complex.rules.final_decision"],
                        policy: `package complex.rules

import rego.v1

default final_decision := {}

# Complex decision making with multiple interdependent rules
final_decision := {
    "allowed": allowed,
    "risk_level": risk_level,
    "required_actions": required_actions,
    "metadata": decision_metadata
}

# Main authorization logic
allowed if {
    not high_risk_user
    not suspicious_activity
    valid_time_window
    resource_available
}

# Risk assessment
risk_level := "high" if high_risk_user
risk_level := "medium" if {
    not high_risk_user
    suspicious_activity
}
risk_level := "low" if {
    not high_risk_user
    not suspicious_activity
}

# High risk user determination
high_risk_user if {
    failed_logins_count > 5
}

high_risk_user if {
    input.user.id in input.blocked_users
}

high_risk_user if {
    geo_anomaly
}

# Suspicious activity detection
suspicious_activity if unusual_access_pattern
suspicious_activity if rapid_requests
suspicious_activity if privilege_escalation_attempt

# Helper rules
failed_logins_count := count([l | l := input.login_attempts[input.user.id][_]; l.status == "failed"])

geo_anomaly if {
    user_location := input.context.geo.country
    usual_locations := {l | l := input.user_locations[input.user.id][_]}
    not user_location in usual_locations
}

unusual_access_pattern if {
    current_hour := time.clock([time.now_ns(), "UTC"])[0]
    current_hour < 6
    current_hour > 22
}

rapid_requests if {
    recent_requests := [r | r := input.recent_requests[input.user.id][_]; r.timestamp > time.now_ns() - 300000000000]  # 5 minutes
    count(recent_requests) > 100
}

privilege_escalation_attempt if {
    input.requested_permissions[_] in ["admin", "super_admin"]
    not input.user.role in ["admin", "super_admin"]
}

valid_time_window if {
    current_hour := time.clock([time.now_ns(), "UTC"])[0]
    current_hour >= input.business_hours.start
    current_hour <= input.business_hours.end
}

resource_available if {
    not input.resource.id in input.maintenance_resources
}

# Required actions based on decision
required_actions contains "mfa_verification" if {
    allowed
    risk_level in ["medium", "high"]
}

required_actions contains "manager_approval" if {
    allowed
    privilege_escalation_attempt
}

required_actions contains "security_review" if high_risk_user

# Decision metadata
decision_metadata := {
    "timestamp": time.now_ns(),
    "factors": {
        "high_risk_user": high_risk_user,
        "suspicious_activity": suspicious_activity,
        "failed_logins": failed_logins_count,
        "geo_anomaly": geo_anomaly
    }
}`,
                        input: `{
    "user": {"id": "user123", "role": "user"},
    "resource": {"id": "resource456", "type": "document"},
    "context": {"geo": {"country": "US"}},
    "requested_permissions": ["read", "write"],
    "blocked_users": ["baduser1", "baduser2"],
    "login_attempts": {
        "user123": [
            {"timestamp": 1630000000, "status": "success"},
            {"timestamp": 1630000060, "status": "failed"}
        ]
    },
    "user_locations": {
        "user123": ["US", "CA"]
    },
    "recent_requests": {
        "user123": [
            {"timestamp": 1630000000000000000, "endpoint": "/api/data"}
        ]
    },
    "business_hours": {
        "start": 9,
        "end": 17
    },
    "maintenance_resources": ["resource999"]
}`,
                        data: '{}'
                    }
                ]
            },
            policies: [] // Legacy flat array for compatibility
        };
        
        // Populate legacy policies array from categories
        Object.values(this.examples.categories).forEach(categoryExamples => {
            this.examples.policies.push(...categoryExamples);
        });
        
        this.init();
    }

    async init() {
        this.updateStatus('Initializing Monaco Editor...');
        await this.initMonaco();
        
        this.updateStatus('Loading WASM module...');
        await this.loadWASM();
        
        this.setupEventListeners();
        this.loadSettings();
        
        // Load default example (Simple Conditions from Basic category)
        const firstExample = this.examples.categories['Basic'][0];
        this.loadExample(firstExample);
        
        this.updateStatus('Ready');
    }

    async initMonaco() {
        return new Promise((resolve) => {
            require.config({ paths: { vs: 'https://unpkg.com/monaco-editor@0.44.0/min/vs' } });
            require(['vs/editor/editor.main'], () => {
                this.setupMonacoEditors();
                resolve();
            });
        });
    }

    setupMonacoEditors() {
        // Register Rego language
        monaco.languages.register({ id: 'rego' });
        
        // Define Rego syntax highlighting
        monaco.languages.setMonarchTokensProvider('rego', {
            tokenizer: {
                root: [
                    [/\b(package|import|as|default|if|else|not|some|every|with)\b/, 'keyword'],
                    [/\b(true|false|null)\b/, 'constant'],
                    [/\b\d+(\.\d+)?\b/, 'number'],
                    [/"([^"\\]|\\.)*"/, 'string'],
                    [/`([^`\\]|\\.)*`/, 'string'],
                    [/#.*$/, 'comment'],
                    [/[{}\[\]()]/, 'bracket'],
                    [/[<>]=?|[!=]=?|&&|\|\||\+|-|\*|\/|%/, 'operator'],
                    [/[a-zA-Z_][a-zA-Z0-9_]*/, 'identifier']
                ]
            }
        });

        // Setup policy editor
        this.editors.policy = monaco.editor.create(document.getElementById('policy-editor'), {
            value: '',
            language: 'rego',
            theme: this.settings.theme,
            wordWrap: this.settings.wordWrap ? 'on' : 'off',
            minimap: { enabled: this.settings.minimap },
            fontSize: 14,
            lineNumbers: 'on',
            roundedSelection: false,
            scrollBeyondLastLine: false,
            automaticLayout: true
        });

        // Setup input editor (JSON)
        this.editors.input = monaco.editor.create(document.getElementById('input-editor'), {
            value: '{}',
            language: 'json',
            theme: this.settings.theme,
            wordWrap: this.settings.wordWrap ? 'on' : 'off',
            minimap: { enabled: false },
            fontSize: 13,
            lineNumbers: 'on',
            automaticLayout: true
        });

        // Setup data editor (JSON)
        this.editors.data = monaco.editor.create(document.getElementById('data-editor'), {
            value: '{}',
            language: 'json',
            theme: this.settings.theme,
            wordWrap: this.settings.wordWrap ? 'on' : 'off',
            minimap: { enabled: false },
            fontSize: 13,
            lineNumbers: 'on',
            automaticLayout: true
        });

        // Add change listeners
        this.editors.policy.onDidChangeModelContent(() => {
            if (this.settings.autoEvaluate) {
                this.debounce(() => this.compile(), 1000);
            }
        });

        this.editors.input.onDidChangeModelContent(() => {
            if (this.settings.autoEvaluate) {
                this.debounce(() => this.evaluate(), 500);
            }
        });

        this.editors.data.onDidChangeModelContent(() => {
            if (this.settings.autoEvaluate) {
                this.debounce(() => this.evaluate(), 500);
            }
        });
    }

    async loadWASM() {
        try {
            this.updateStatus('Loading WASM module...');
            
            // Import the real WASM module
            const wasmModule = await import('./wasm/regorusjs.js');
            await wasmModule.default();
            
            // Import the classes and functions we need
            const { PolicyModule, generateAssemblyListing, compileToRvmProgram, RegoVM } = wasmModule;
            
            this.wasmModule = {
                PolicyModule,
                generateAssemblyListing,
                compileToRvmProgram,
                RegoVM
            };
            
            this.updateStatus('WASM module loaded successfully');
            this.isWasmLoaded = true;
            
        } catch (error) {
            console.error('Failed to load WASM module:', error);
            const errorMessage = error.message || error.toString() || 'Unknown WASM loading error';
            this.updateStatus(`Failed to load WASM module: ${errorMessage}`);
            throw error;
        }
    }

    setupEventListeners() {
        // Header buttons
        document.getElementById('compile-btn').addEventListener('click', () => this.compile());
        document.getElementById('evaluate-btn').addEventListener('click', () => this.evaluate());
        document.getElementById('benchmark-btn').addEventListener('click', () => this.runBenchmark());
        document.getElementById('settings-btn').addEventListener('click', () => this.showSettings());
        
        // Panel actions
        document.getElementById('policy-examples-btn').addEventListener('click', () => this.showExamples('policies'));
        document.getElementById('policy-clear-btn').addEventListener('click', () => this.clearPolicy());
        
        document.getElementById('assembly-copy-btn').addEventListener('click', () => this.copyAssembly());
        document.getElementById('assembly-format').addEventListener('change', (e) => {
            this.settings.assemblyFormat = e.target.value;
            this.refreshAssembly();
        });
        
        document.getElementById('data-examples-btn').addEventListener('click', () => this.showExamples('data'));
        
        // Tab switching
        document.querySelectorAll('.tab-button').forEach(button => {
            button.addEventListener('click', (e) => {
                const tab = e.target.dataset.tab;
                this.switchTab(tab);
            });
        });
        
        // Modal handling
        document.querySelectorAll('.modal-close').forEach(close => {
            close.addEventListener('click', (e) => {
                e.target.closest('.modal').classList.remove('show');
            });
        });
        
        // Click outside modal to close
        document.querySelectorAll('.modal').forEach(modal => {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    modal.classList.remove('show');
                }
            });
        });
        
        // Entry points management
        document.getElementById('add-entry-point').addEventListener('click', () => this.addEntryPoint());
        this.setupEntryPointListeners();
    }

    async compile() {
        if (!this.isWasmLoaded || !this.wasmModule) {
            this.updateStatus('WASM module not loaded');
            this.showAssembly('; Error: WASM module not loaded');
            return;
        }

        const policy = this.editors.policy.getValue().trim();
        if (!policy) {
            this.showAssembly(''); 
            this.currentProgram = null;
            return;
        }

        try {
            this.updateStatus('Compiling policy...');
            const startTime = performance.now();
            
            // Get data and entry points
            const data = this.editors.data.getValue().trim() || '{}';
            const entryPoints = this.getEntryPoints();
            
            // Create policy module
            const policyModule = new this.wasmModule.PolicyModule('policy.rego', policy);
            
            // Use the standalone compilation function
            this.currentProgram = this.wasmModule.compileToRvmProgram(
                data,
                [policyModule],
                entryPoints
            );
            
            // Generate assembly listing
            const format = document.getElementById('assembly-format').value || 'readable';
            const assembly = this.wasmModule.generateAssemblyListing(
                this.currentProgram,
                format,
                null // Use default config
            );
            
            const endTime = performance.now();
            const compilationTime = (endTime - startTime).toFixed(2);
            
            this.showAssembly(assembly);
            this.updateInstructionCount(this.currentProgram.getInstructionCount());
            this.updateStatus(`Compiled successfully (${compilationTime}ms)`);
            
        } catch (error) {
            console.error('Compilation error:', error);
            const errorMessage = error.message || error.toString() || 'Unknown compilation error';
            this.updateStatus(`Compilation failed: ${errorMessage}`);
            this.showAssembly(`; Compilation Error:\n; ${errorMessage}`);
            this.currentProgram = null;
        }
    }

    extractEntryPoint(policy) {
        // Simple heuristic to find entry point
        const lines = policy.split('\n');
        for (const line of lines) {
            const trimmed = line.trim();
            if (trimmed.startsWith('allow') && !trimmed.includes('{')) {
                return 'data.' + this.extractPackageName(policy) + '.allow';
            }
            if (trimmed.match(/^[a-zA-Z_][a-zA-Z0-9_]*\s*[:=]/)) {
                const ruleName = trimmed.split(/[:=]/)[0].trim();
                return 'data.' + this.extractPackageName(policy) + '.' + ruleName;
            }
        }
        return 'data.' + this.extractPackageName(policy) + '.allow'; // fallback
    }

    extractPackageName(policy) {
        const packageMatch = policy.match(/package\s+([a-zA-Z_][a-zA-Z0-9_.]*)/);
        return packageMatch ? packageMatch[1] : 'main';
    }

    generateMockAssembly(policy) {
        // This is a mock function - in reality, this would use the WASM module
        const lines = policy.split('\n').filter(line => line.trim());
        let assembly = '; RVM Assembly - Mock Generated\n';
        assembly += '; Instructions: ~' + (lines.length * 3) + ', Literals: ~' + (lines.length) + '\n';
        assembly += ';\n';
        assembly += '; LITERALS (JSON values):\n';
        assembly += ';   L 0: "package"\n';
        assembly += ';   L 1: "allow"\n';
        assembly += ';   L 2: true\n';
        assembly += ';\n';
        assembly += '; ===== RULE: data.authz.allow =====\n';
        assembly += '000: RuleInit     data.authz.allow → r0 {\n';
        assembly += '001:     LoadInput    r1 ← input                   ; Load global input document\n';
        assembly += '002:     IndexLiteral r2 ← r1[L0]                 ; Index with literal key: r1["user"]\n';
        assembly += '003:     IndexLiteral r3 ← r2[L1]                 ; Index with literal key: r2["role"]\n';
        assembly += '004:     Load         r4 ← L2                    ; Load literal: "admin"\n';
        assembly += '005:     Eq           r5 ← (r3 = r4)             ; Equality test: r3 == r4\n';
        assembly += '006:     AssertCondition assert r5                ; Assert r5 is true (exit if false/undefined)\n';
        assembly += '007:     LoadTrue     r0 ← true                  ; Load boolean constant true\n';
        assembly += '008: } return from rule                           ; End of rule evaluation\n';
        
        return assembly;
    }

    countInstructions(assembly) {
        const lines = assembly.split('\n');
        let count = 0;
        for (const line of lines) {
            if (/^\d{3}:/.test(line.trim())) {
                count++;
            }
        }
        return count;
    }

    showAssembly(assembly) {
        const display = document.getElementById('assembly-display');
        display.textContent = assembly;
        
        // Apply syntax highlighting
        this.highlightAssembly(display);
    }

    highlightAssembly(element) {
        const text = element.textContent;
        let highlighted = text
            .replace(/(\b\w+:)/g, '<span class="assembly-instruction">$1</span>')
            .replace(/(r\d+)/g, '<span class="assembly-register">$1</span>')
            .replace(/(L\d+)/g, '<span class="assembly-literal">$1</span>')
            .replace(/(;.*$)/gm, '<span class="assembly-comment">$1</span>')
            .replace(/(^\d{3}:)/gm, '<span class="assembly-address">$1</span>')
            .replace(/(===== RULE: .* =====)/g, '<span class="assembly-rule">$1</span>');
        
        element.innerHTML = highlighted;
    }

    async evaluate() {
        const policy = this.editors.policy.getValue().trim();
        const input = this.editors.input.getValue().trim() || '{}';
        const data = this.editors.data.getValue().trim() || '{}';
        
        if (!policy) {
            this.showResults('No policy to evaluate', 'error');
            return;
        }

        // Compile first if needed
        if (!this.currentProgram) {
            await this.compile();
            if (!this.currentProgram) {
                this.showResults('Compilation failed - cannot evaluate', 'error');
                return;
            }
        }

        try {
            this.updateStatus('Evaluating policy...');
            const startTime = performance.now();
            
            // Create VM and load program
            const vm = new this.wasmModule.RegoVM();
            vm.loadProgram(this.currentProgram);
            vm.setInput(input);
            vm.setData(data);
            
            // Execute and get result
            const resultJson = vm.execute();
            const result = JSON.parse(resultJson);
            
            const endTime = performance.now();
            const evaluationTime = (endTime - startTime).toFixed(2);
            
            // Format results with timing information
            const formattedResult = this.formatEvaluationResult(result, input, data, policy, evaluationTime);
            this.showResults(formattedResult, 'success');
            
            if (this.settings.showTiming) {
                this.updateExecutionTime(`${evaluationTime}ms`);
            }
            
            this.updateStatus(`Evaluation completed (${evaluationTime}ms)`);
            
        } catch (error) {
            console.error('Evaluation error:', error);
            this.updateStatus(`Evaluation failed: ${error.message}`);
            this.showResults(`Error: ${error.message}`, 'error');
        }
    }

    async runBenchmark() {
        const policy = this.editors.policy.getValue().trim();
        const input = this.editors.input.getValue().trim() || '{}';
        const data = this.editors.data.getValue().trim() || '{}';
        
        if (!policy) {
            this.showResults('No policy to benchmark', 'error');
            return;
        }

        // Compile first if needed
        if (!this.currentProgram) {
            await this.compile();
            if (!this.currentProgram) {
                this.showResults('Compilation failed - cannot benchmark', 'error');
                return;
            }
        }

        try {
            this.updateStatus('Running benchmark...');
            
            const iterations = [1, 10, 100, 1000];
            const results = [];
            
            for (const iterCount of iterations) {
                // Warmup run
                const warmupVm = new this.wasmModule.RegoVM();
                warmupVm.loadProgram(this.currentProgram);
                warmupVm.setInput(input);
                warmupVm.setData(data);
                warmupVm.execute();
                
                // Benchmark run
                const startTime = performance.now();
                
                for (let i = 0; i < iterCount; i++) {
                    const vm = new this.wasmModule.RegoVM();
                    vm.loadProgram(this.currentProgram);
                    vm.setInput(input);
                    vm.setData(data);
                    const result = vm.execute();
                    
                    // Parse result to ensure complete execution
                    JSON.parse(result);
                }
                
                const endTime = performance.now();
                const totalTime = endTime - startTime;
                const avgTime = totalTime / iterCount;
                
                results.push({
                    iterations: iterCount,
                    totalTime: totalTime.toFixed(2),
                    avgTime: avgTime.toFixed(3),
                    opsPerSec: (1000 / avgTime).toFixed(0)
                });
                
                this.updateStatus(`Benchmarking ${iterCount} iterations...`);
                
                // Allow UI to update
                await new Promise(resolve => setTimeout(resolve, 10));
            }
            
            // Get single execution result for display
            const vm = new this.wasmModule.RegoVM();
            vm.loadProgram(this.currentProgram);
            vm.setInput(input);
            vm.setData(data);
            const resultJson = vm.execute();
            const result = JSON.parse(resultJson);
            
            // Format results with benchmark data
            const formattedResult = this.formatBenchmarkResult(result, input, data, policy, results);
            this.showResults(formattedResult, 'success');
            
            this.updateStatus('Benchmark completed');
            
        } catch (error) {
            console.error('Benchmark error:', error);
            this.updateStatus(`Benchmark failed: ${error.message}`);
            this.showResults(`Error: ${error.message}`, 'error');
        }
    }

    formatEvaluationResult(result, input, data, policy, evaluationTime = null) {
        let output = '# Evaluation Results\n\n';
        
        // Extract main result vs debug/trace information
        let mainResult = result;
        let debugInfo = null;
        let printStatements = null;
        
        // If result is an object with debug info, separate it
        if (typeof result === 'object' && result !== null) {
            if (result.result !== undefined) {
                mainResult = result.result;
                debugInfo = { ...result };
                delete debugInfo.result;
            } else if (result.value !== undefined) {
                mainResult = result.value;
                debugInfo = { ...result };
                delete debugInfo.value;
            }
            
            // Extract print statements if present
            if (result.prints) {
                printStatements = result.prints;
                if (debugInfo) delete debugInfo.prints;
            }
        }
        
        // Display main policy result in highlighted JSON box
        output += `## Policy Result\n`;
        output += '<div class="json-result-box">\n';
        output += '<pre><code class="language-json">';
        output += JSON.stringify(mainResult, null, 2);
        output += '</code></pre>\n';
        output += '</div>\n\n';
        
        // Show timing information if provided
        if (evaluationTime !== null) {
            output += `## Execution Time\n`;
            output += '<div class="timing-info">\n';
            output += `<span class="timing-label">Evaluation Time:</span> <span class="timing-value">${evaluationTime}ms</span>\n`;
            output += '</div>\n\n';
        }
        
        // Show print statements and logs in separate section
        if (printStatements && printStatements.length > 0) {
            output += `## Execution Logs\n`;
            output += '<div class="logs-container">\n';
            printStatements.forEach(print => {
                // Handle newlines properly in print statements
                const cleanPrint = print.replace(/\\n/g, '\n').replace(/\\t/g, '\t');
                output += `<div class="log-entry">${cleanPrint}</div>\n`;
            });
            output += '</div>\n\n';
        }
        
        // Show debug/trace information if present
        if (debugInfo && Object.keys(debugInfo).length > 0) {
            output += `## Debug Information\n`;
            output += '```json\n';
            output += JSON.stringify(debugInfo, null, 2);
            output += '\n```\n\n';
        }
        
        // Show program information if available
        if (this.currentProgram) {
            output += `## Program Information\n`;
            output += '```\n';
            const programInfo = this.currentProgram.getProgramInfo();
            // Handle newlines in program info
            const cleanProgramInfo = programInfo.replace(/\\n/g, '\n');
            output += cleanProgramInfo;
            output += '```\n\n';
        }
        
        // Show input and data in collapsed sections for reference
        output += `## Input Data\n`;
        output += '<details><summary>Click to expand input</summary>\n\n';
        output += '```json\n';
        output += JSON.stringify(JSON.parse(input), null, 2);
        output += '\n```\n</details>\n\n';
        
        output += `## Policy Data\n`;
        output += '<details><summary>Click to expand data</summary>\n\n';
        output += '```json\n';
        output += JSON.stringify(JSON.parse(data), null, 2);
        output += '\n```\n</details>\n';
        
        return output;
    }

    formatBenchmarkResult(result, input, data, policy, benchmarkResults) {
        let output = '# Benchmark Results\n\n';
        
        // Extract main result
        let mainResult = result;
        if (typeof result === 'object' && result !== null) {
            if (result.result !== undefined) {
                mainResult = result.result;
            } else if (result.value !== undefined) {
                mainResult = result.value;
            }
        }
        
        // Display main policy result in highlighted JSON box
        output += `## Policy Result\n`;
        output += '<div class="json-result-box">\n';
        output += '<pre><code class="language-json">';
        output += JSON.stringify(mainResult, null, 2);
        output += '</code></pre>\n';
        output += '</div>\n\n';
        
        // Display benchmark results
        output += `## Performance Benchmark\n`;
        output += '<div class="benchmark-table">\n';
        output += '<table>\n';
        output += '<thead>\n';
        output += '<tr><th>Iterations</th><th>Total Time</th><th>Avg Time</th><th>Ops/Sec</th></tr>\n';
        output += '</thead>\n';
        output += '<tbody>\n';
        benchmarkResults.forEach(bench => {
            output += `<tr><td>${bench.iterations}</td><td>${bench.totalTime}ms</td><td>${bench.avgTime}ms</td><td>${bench.opsPerSec}</td></tr>\n`;
        });
        output += '</tbody>\n';
        output += '</table>\n';
        output += '</div>\n\n';
        
        // Show fastest execution stats
        const fastest = benchmarkResults[benchmarkResults.length - 1];
        output += '<div class="performance-summary">\n';
        output += `<strong>Best Performance:</strong> ${fastest.avgTime}ms per execution (${fastest.opsPerSec} ops/sec)\n`;
        output += '</div>\n';
        
        return output;
    }

    generateMockEvaluation(policy, input, data) {
        // Mock evaluation result
        const inputObj = JSON.parse(input);
        
        let result = '# Evaluation Results\n\n';
        
        if (policy.includes('allow')) {
            const userRole = inputObj.user?.role || 'unknown';
            const allowed = userRole === 'admin' || (inputObj.user?.id === inputObj.resource?.owner);
            
            result += `## Policy Decision\n`;
            result += `**allow**: ${allowed}\n\n`;
            
            result += `## Trace\n`;
            result += `1. Loaded input: user.role = "${userRole}"\n`;
            result += `2. Evaluated rule: data.authz.allow\n`;
            result += `3. Result: ${allowed}\n\n`;
        }
        
        result += `## Input\n\`\`\`json\n${JSON.stringify(JSON.parse(input), null, 2)}\n\`\`\`\n\n`;
        result += `## Data\n\`\`\`json\n${JSON.stringify(JSON.parse(data), null, 2)}\n\`\`\`\n`;
        
        return result;
    }

    showResults(content, type = 'success') {
        const display = document.getElementById('results-display');
        
        // For markdown content, render as HTML with basic formatting
        if (content.includes('# ') || content.includes('## ') || content.includes('```')) {
            // Simple markdown-to-HTML conversion
            let htmlContent = content
                .replace(/^# (.+)$/gm, '<h1>$1</h1>')
                .replace(/^## (.+)$/gm, '<h2>$1</h2>')
                .replace(/^### (.+)$/gm, '<h3>$1</h3>')
                .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
                .replace(/`([^`]+)`/g, '<code>$1</code>')
                .replace(/```(\w+)?\n([\s\S]*?)```/g, '<pre><code class="language-$1">$2</code></pre>')
                .replace(/```\n([\s\S]*?)```/g, '<pre><code>$1</code></pre>')
                .replace(/\n/g, '<br>')
                .replace(/<\/h[1-6]><br>/g, function(match) {
                    return match.replace('<br>', '');
                });
            
            // Handle collapsible details/summary elements
            htmlContent = htmlContent.replace(
                /<details><summary>(.+?)<\/summary><br><br>/g, 
                '<details><summary>$1</summary><div class="details-content">'
            );
            htmlContent = htmlContent.replace(/<\/details>/g, '</div></details>');
            
            display.innerHTML = htmlContent;
        } else {
            display.textContent = content;
        }
        
        display.className = `results-output ${type}`;
        
        // Switch to results tab
        this.switchTab('results');
    }

    switchTab(tabName) {
        // Update tab buttons
        document.querySelectorAll('.tab-button').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.tab === tabName);
        });
        
        // Update tab panes
        document.querySelectorAll('.tab-pane').forEach(pane => {
            pane.classList.toggle('active', pane.id === `${tabName}-tab`);
        });
    }

    showSettings() {
        const modal = document.getElementById('settings-modal');
        
        // Update settings form with current values
        document.getElementById('setting-word-wrap').checked = this.settings.wordWrap;
        document.getElementById('setting-minimap').checked = this.settings.minimap;
        document.getElementById('setting-theme').value = this.settings.theme;
        document.getElementById('setting-show-addresses').checked = this.settings.showAddresses;
        document.getElementById('setting-show-comments').checked = this.settings.showComments;
        document.getElementById('setting-auto-evaluate').checked = this.settings.autoEvaluate;
        document.getElementById('setting-show-timing').checked = this.settings.showTiming;
        
        // Add event listeners for settings
        this.setupSettingsListeners();
        
        modal.classList.add('show');
    }

    setupSettingsListeners() {
        const applySettings = () => {
            this.settings.wordWrap = document.getElementById('setting-word-wrap').checked;
            this.settings.minimap = document.getElementById('setting-minimap').checked;
            this.settings.theme = document.getElementById('setting-theme').value;
            this.settings.showAddresses = document.getElementById('setting-show-addresses').checked;
            this.settings.showComments = document.getElementById('setting-show-comments').checked;
            this.settings.autoEvaluate = document.getElementById('setting-auto-evaluate').checked;
            this.settings.showTiming = document.getElementById('setting-show-timing').checked;
            
            this.applyEditorSettings();
            this.saveSettings();
        };
        
        document.querySelectorAll('#settings-modal input, #settings-modal select').forEach(input => {
            input.removeEventListener('change', applySettings); // Prevent duplicate listeners
            input.addEventListener('change', applySettings);
        });
    }

    applyEditorSettings() {
        Object.values(this.editors).forEach(editor => {
            editor.updateOptions({
                theme: this.settings.theme,
                wordWrap: this.settings.wordWrap ? 'on' : 'off',
                minimap: { enabled: this.settings.minimap }
            });
        });
    }

    showExamples(type) {
        const modal = document.getElementById('examples-modal');
        const title = document.getElementById('examples-modal-title');
        const list = document.getElementById('examples-list');
        
        title.textContent = `Example ${type === 'policies' ? 'Policies' : 'Data'}`;
        
        list.innerHTML = '';
        
        if (type === 'policies') {
            // Display examples organized by category
            Object.entries(this.examples.categories).forEach(([categoryName, examples]) => {
                // Create category header
                const categoryHeader = document.createElement('div');
                categoryHeader.className = 'example-category-header';
                categoryHeader.innerHTML = `<h3>${categoryName}</h3>`;
                list.appendChild(categoryHeader);
                
                // Create examples in this category
                examples.forEach((example, index) => {
                    const item = document.createElement('div');
                    item.className = 'example-item';
                    item.innerHTML = `
                        <h4>${example.name}</h4>
                        <p>${example.description}</p>
                        <div class="example-meta">
                            <span class="category-tag">${categoryName}</span>
                        </div>
                    `;
                    item.addEventListener('click', () => {
                        this.loadExample(example);
                        modal.classList.remove('show');
                    });
                    list.appendChild(item);
                });
            });
        }
        
        modal.classList.add('show');
    }

    loadExample(example) {
        this.editors.policy.setValue(example.policy);
        this.editors.input.setValue(example.input);
        this.editors.data.setValue(example.data);
        
        // Update entry points if provided
        if (example.entryPoints) {
            this.entryPoints = [...example.entryPoints];
            this.updateEntryPointsUI();
            this.saveEntryPoints();
        }
        
        this.updateStatus(`Loaded example: ${example.name}`);
    }

    clearPolicy() {
        if (confirm('Clear the current policy?')) {
            this.editors.policy.setValue('');
            this.showAssembly('');
            this.updateStatus('Policy cleared');
        }
    }

    refreshAssembly() {
        if (this.currentProgram) {
            const format = document.getElementById('assembly-format').value || 'readable';
            const assembly = this.currentProgram.getAssemblyListing(format);
            this.showAssembly(assembly);
        }
    }

    copyAssembly() {
        const assembly = document.getElementById('assembly-display').textContent;
        if (assembly) {
            navigator.clipboard.writeText(assembly).then(() => {
                this.updateStatus('Assembly copied to clipboard');
            }).catch(err => {
                console.error('Failed to copy:', err);
            });
        }
    }

    // Utility methods
    debounce(func, wait) {
        clearTimeout(this.debounceTimer);
        this.debounceTimer = setTimeout(func, wait);
    }

    updateStatus(message) {
        document.getElementById('status-text').textContent = message;
    }

    updateVMStatus(message) {
        document.getElementById('vm-status').textContent = `VM: ${message}`;
    }

    updateExecutionTime(time) {
        document.getElementById('execution-time').textContent = `Exec: ${time}`;
    }

    updateInstructionCount(count) {
        document.getElementById('instruction-count').textContent = `Instructions: ${count}`;
    }

    saveSettings() {
        localStorage.setItem('rvmPlaygroundSettings', JSON.stringify(this.settings));
    }

    loadSettings() {
        const saved = localStorage.getItem('rvmPlaygroundSettings');
        if (saved) {
            this.settings = { ...this.settings, ...JSON.parse(saved) };
            this.applyEditorSettings();
        }
        
        const savedEntryPoints = localStorage.getItem('rvmPlaygroundEntryPoints');
        if (savedEntryPoints) {
            this.entryPoints = JSON.parse(savedEntryPoints);
            this.updateEntryPointsUI();
        }
    }
    
    setupEntryPointListeners() {
        const container = document.getElementById('entry-points-list');
        container.addEventListener('click', (e) => {
            if (e.target.classList.contains('entry-point-remove')) {
                this.removeEntryPoint(e.target);
            }
        });
        
        container.addEventListener('input', (e) => {
            if (e.target.classList.contains('entry-point-input')) {
                this.updateEntryPointsFromUI();
            }
        });
    }
    
    addEntryPoint() {
        const container = document.getElementById('entry-points-list');
        const newItem = document.createElement('div');
        newItem.className = 'entry-point-item';
        newItem.innerHTML = `
            <input type="text" class="entry-point-input" value="" placeholder="e.g., data.package.rule">
            <button class="entry-point-remove" title="Remove entry point">×</button>
        `;
        container.appendChild(newItem);
        newItem.querySelector('input').focus();
        this.updateEntryPointsFromUI();
    }
    
    removeEntryPoint(button) {
        const item = button.parentElement;
        const container = document.getElementById('entry-points-list');
        if (container.children.length > 1) {
            item.remove();
            this.updateEntryPointsFromUI();
        }
    }
    
    updateEntryPointsFromUI() {
        const inputs = document.querySelectorAll('.entry-point-input');
        this.entryPoints = Array.from(inputs)
            .map(input => input.value.trim())
            .filter(value => value.length > 0);
        
        this.saveEntryPoints();
    }
    
    saveEntryPoints() {
        localStorage.setItem('rvmPlaygroundEntryPoints', JSON.stringify(this.entryPoints));
    }
    
    updateEntryPointsUI() {
        const container = document.getElementById('entry-points-list');
        container.innerHTML = '';
        
        this.entryPoints.forEach(entryPoint => {
            const item = document.createElement('div');
            item.className = 'entry-point-item';
            item.innerHTML = `
                <input type="text" class="entry-point-input" value="${entryPoint}" placeholder="e.g., data.package.rule">
                <button class="entry-point-remove" title="Remove entry point">×</button>
            `;
            container.appendChild(item);
        });
        
        if (this.entryPoints.length === 0) {
            this.addEntryPoint();
        }
    }
    
    getEntryPoints() {
        this.updateEntryPointsFromUI();
        return this.entryPoints.length > 0 ? this.entryPoints : ['data.main.allow'];
    }
}

// Initialize the playground when the DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.playground = new RVMPlayground();
});