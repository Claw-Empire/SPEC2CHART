# User Story Map

Map user activities across the journey, with stories organized by priority below each activity.

## Config
flow = LR
spacing = 70

## User Journey
- [j1] Discover {icon:🔍} {done}
  User first encounters the product.
- [j2] Sign Up {icon:📝} {done}
  User creates an account.
- [j3] Onboard {icon:🎓} {wip}
  User learns core features.
- [j4] Core Use {icon:⚙️} {wip}
  User does the main thing.
- [j5] Share {icon:📤}
  User shares or exports results.
- [j6] Return {icon:🔄}
  User comes back for more value.

## Must Have
- [m1] Find via search {icon:🔍} {done}
- [m2] Landing page {icon:📄} {done}
- [m3] Email signup {icon:📧} {done}
- [m4] OAuth login {icon:🔑} {done}
- [m5] Welcome email {icon:📨} {wip}
- [m6] Empty state hint {icon:💡} {wip}
- [m7] Create first item {icon:➕} {wip}
- [m8] Edit item {icon:✏️}
- [m9] Share link {icon:🔗}
- [m10] Email reminder {icon:⏰}

## Should Have
- [s1] SEO content {icon:📊}
- [s2] Social OAuth {icon:👥}
- [s3] Guided tour {icon:🗺}
- [s4] Bulk actions {icon:⚡}
- [s5] Export {icon:📁}
- [s6] Push notifications {icon:🔔}

## Could Have
- [c1] Ads {icon:📢}
- [c2] Analytics dashboard {icon:📈}
- [c3] Collaborators {icon:🤝}
- [c4] API access {icon:🔌}
- [c5] Integrations {icon:🔗}
- [c6] Mobile app {icon:📱}

## Flow
j1 --> j2 --> j3 --> j4 --> j5 --> j6
j1 --> m1
j1 --> m2
j2 --> m3
j2 --> m4
j3 --> m5
j3 --> m6
j4 --> m7
j4 --> m8
j5 --> m9
j6 --> m10

## Summary
User Story Map: organize stories by user journey activities with must-have/should-have/could-have layers.
