# Rest FlowType

```json
{
  "identifier": "REST"
  "name": [
    {
      "code": "en-US",
      "content": "Rest Endpoint"
    }
  ],
  "description": [
    {
      "code": "en-US",
      "content": "A REST API is a web service that lets clients interact with data on a server using standard HTTP methods like GET, POST, PUT, and DELETE, usually returning results in JSON format."
    }
  ],
  "settings": [],
  "input_type_identifier": "HTTP_REQUEST_OBJECT",
  "return_type_identifier": "HTTP_RESPONSE_OBJECT"
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
          "items": [ "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"]
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
    "variant": "ARRAY",
    "identifier": "HTTP_HEADER_MAP",
    "name": [
      {
        "code": "en-US",
        "content": "HTTP Headers"
      }
    ],
    "rules": [
      {
        "contains_type": {
          "type": "HTTP_HEADER_ENTRY"
        }
      }
    ],
    "parent_type_identifier": "ARRAY"
  },
  {
    "variant": "OBJECT",
    "identifier": "HTTP_HEADER_ENTRY",
    "name": [
      {
        "code": "en-US",
        "content": "HTTP Header Entry"
      }
    ],
    "rules": [
      {
        "contains_key": {
          "key": "key",
          "type": "TEXT"
        }
      },
      {
        "contains_key": {
          "key": "value",
          "type": "TEXT"
        }
      }
    ],
    "parent_type_identifier": "OBJECT"
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
          "type": "HTTP_HEADER_MAP"
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
          "type": "HTTP_HEADER_MAP"
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
