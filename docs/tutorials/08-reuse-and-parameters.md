# Chapter 8: Reuse nodes and parameters

Templates and instances let several graph nodes reuse one definition while
parameters keep environment-specific values outside the action structure.

## 8.1 Define parameters

Parameter definitions belong to entries in `instances`:

```json
"EnterAccount": {
  "node": "EnterValue",
  "params": [
    {
      "key": "account",
      "label": "Account",
      "type": "text",
      "default": "demo"
    },
    {
      "key": "remember",
      "label": "Remember account",
      "type": "switch",
      "default": false
    }
  ]
}
```

Use placeholders in supported strings:

```json
{ "type": "InputText", "input_text": "{{account}}" }
```

## 8.2 Understand value precedence

The effective value is applied in this order:

1. Parameter definition default.
2. Instance `param_values`.
3. Explicit run value entered by the user or caller.

Later sources override earlier ones. The UI displays the effective defaults so
the value used by a normal run and Node Test can be reviewed before execution.

## 8.3 Reuse a node definition

```json
{
  "nodes": {
    "EnterValue": {
      "recognition": {
        "type": "OCR",
        "expected": "{{label}}"
      },
      "actions": [
        { "type": "Click" },
        { "type": "InputText", "input_text": "{{value}}" }
      ]
    }
  },
  "instances": {
    "EnterAccount": {
      "node": "EnterValue",
      "params": [
        { "key": "label", "label": "Field label", "type": "text" },
        { "key": "value", "label": "Value", "type": "text" }
      ],
      "param_values": {
        "label": "Account",
        "value": "demo"
      }
    },
    "EnterPassword": {
      "node": "EnterValue",
      "params": [
        { "key": "label", "label": "Field label", "type": "text" },
        { "key": "value", "label": "Value", "type": "password" }
      ],
      "param_values": {
        "label": "Password"
      }
    }
  }
}
```

The Mermaid graph uses `EnterAccount` and `EnterPassword`; both resolve through
`EnterValue`.

## 8.4 Protect sensitive values

Do not put real passwords or tokens in committed defaults. Supply them as
explicit run values. Patrol action evidence records input character counts,
not the input text.

Next: [Test and debug pipelines](09-testing-and-debugging.md).
