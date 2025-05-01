# Rest FlowType

```json
{
  "flow_type_identifier": "REST"
  "name": [
    {
      "code": "en-US",
      "content": "Rest Endpoint"
    }
  ],
  "definition": {
    "editable": false,
    "input_type_identifier": "HTTP_REQUEST_OBJECT",
    "return_type_identifier": "HTTP_RESPONSE_OBJECT"
    "settings": []
  }
}
```

## Defined DataTypes

```json
[
  {
    "variant": "TYPE",
    "identifier": "HTTP_METHOD",
    "name": [
      {
        "code": "en-US",
        "content": "HTTP Method",
      }
    ],
    "rules": [
      {
        "item_of_collection": {
          "items": [ "GET", "POST", "PUT", "DELETE", "PATCH"]
        }
      }
    ],
    "parent_type_identifier": null
  },
  {
    "variant": "TYPE",
    "identifier": "HTTP_URL",
    "name": [
      {
        "code": "en-US",
        "content": "HTTP Route",
      }
    ],
    "rules": [
      {
        "regex": {
          "pattern": "/^\/\w+(?:[.:~-]\w+)*(?:\/\w+(?:[.:~-]\w+)*)*$/"
        }
      }
    ],
    "parent_type_identifier": null
  },
  {
    "variant": "OBJECT",
    "identifier": "HTTP_REQUEST_OBJECT",
    "name": [
      {
        "code": "en-US",
        "content": "HTTP Request",
      }
    ],
    "rules": [
      {
        "contains_key": {
          "key": "method",
          "type": "HTTP_METHOD"
        }
      },
      {
        "contains_key": {
          "key": "url",
          "type": "HTTP_URL"
        }
      },
      {
        "contains_key": {
          "key": "body",
          "type": "OBJECT"
        }
      },
      {
        "contains_key": {
          "key": "headers",
          "type": "OBJECT"
        }
      }
    ],
    "parent_type_identifier": "OBJECT"
  },
  {
    "variant": "OBJECT",
    "identifier": "HTTP_RESPONSE_OBJECT",
    "name": [
      {
        "code": "en-US",
        "content": "HTTP Response"
      }
    ],
    "rules": [
      {
        "contains_key": {
          "key": "headers",
          "type": "OBJECT"
        }
      },
      {
        "contains_key": {
          "key": "body",
          "type": "OBJECT"
        }
      }
    ],
    "parent_type_identifier": "OBJECT"
  }
]
```
