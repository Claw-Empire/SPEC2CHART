# User Registration Flow

A flowchart showing the complete user registration and onboarding process.

## Config
bg = dots
flow = TB

## Nodes
- [start] Start {circle} {fill:green}
- [form] Registration Form {sublabel:email + password}
- [validate] Validate Input {diamond} {highlight}
  Checks email format, password strength, and duplicate accounts.
- [email] Send Confirmation Email {fill:teal}
- [confirm] Email Confirmed? {diamond}
- [create] Create Account {fill:blue} {highlight}
- [welcome] Welcome Email {fill:teal}
- [end] End {circle} {fill:green}
- [error] Show Error {fill:red} {note:Inline validation with field-level hints}

## Flow
start -> form
form -> validate
validate -> email {dashed} {note:valid input}
validate -> error {dashed} {note:validation failed}
email -> confirm
confirm -> create {note:link clicked}
confirm -> email {dashed} {note:resend}
create -> welcome
welcome -> end
error -> form {dashed}
