# Presence Canary

- Token is defined by the environment variable `OPERATOR_TOKEN`

## Usage

1. Set up your canary listener at some accessible URL
2. Configure manual actions to send a `POST` request:
   - With the `Authorization` header set to `Bearer $OPERATOR_TOKEN`
   - and a plaintext request body that describes the reason for the ping
3. You can access the URL in a browser to see the 8 latest pings, with timestamps.

### Pinging with cURL

```shell
$ curl \
  -H "Authorization: $OPERATOR_TOKEN" \
  -d "$REASON" \
  "$CANARY_URL"
[...]
```
