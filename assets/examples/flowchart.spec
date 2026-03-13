# User Registration Flow

A flowchart showing the complete user registration and onboarding process.

## Nodes
- [start] Start {circle} {fill:green}
- [form] Registration Form
- [validate] Validate Input {diamond}
- [email] Send Confirmation Email
- [confirm] Email Confirmed? {diamond}
- [create] Create Account {fill:blue}
- [welcome] Welcome Email {fill:teal}
- [end] End {circle} {fill:green}
- [error] Show Error {fill:red}

## Flow
start -> form
form -> validate
validate -> email {dashed}
validate -> error {dashed}
email -> confirm
confirm -> create
confirm -> email {dashed}
create -> welcome
welcome -> end
error -> form
