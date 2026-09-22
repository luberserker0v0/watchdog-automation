```mermaid

flowchart TD
    Start["Start"]
    ArmFirstMiss["ArmFirstMiss"]
    ApplyRecovery["ApplyRecovery"]
    VerifyRecovered["VerifyRecovered"]
    Done["Done"]

    Start --> ArmFirstMiss
    ArmFirstMiss --> ApplyRecovery
    ApplyRecovery --> VerifyRecovered
    VerifyRecovered --> Done
```
