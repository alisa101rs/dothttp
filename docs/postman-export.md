# Environments

## Example

```shell
dothttp export-environment -n env.json -e dev --name example > example.postman_environment.json
```

# Collections

## Example

```shell
dothttp export-collection --name my-collection request-1.http request-2.http > my-collection.postman_collection.json
```

# Compatability between ijhttp and postman

## Dynamic Variables

| ijhttp dynamic variables     | postman alternative | comments                                                                              |
| ---------------------------- | ------------------- | ------------------------------------------------------------------------------------- |
| $uuid                        | `$guid`             |                                                                                       |
| $random.uuid                 | `$randomUUID`       |                                                                                       |
| $timestamp                   | `$timestamp`        | current unix timestamp                                                                |
| $isoTimestamp                | `$isoTimestamp`     |                                                                                       |
| $randomInt                   | `$randomInt`        | random integer between 0 and 1000                                                     |
| $random.integer(from, to)    | custom script       | random integer between `from` and `to`                                                |
| $random.integer              | `$randomInt`        | random integer between `0` and `1000`                                                 |
| $random.float(from, to)      | custom script       | random floating point number between `from` and `to`                                  |
| $random.float                | custom script       | random floating point number between `0.0` and `1000.0`                               |
| $random.alphabetic(length)   | custom script       | sequence of uppercase and lowercase letters of length `length`                        |
| $random.alphanumeric(length) | custom script       | seqence of uppercase and lowercase letters, digits and underscores of length `length` |
| $random.hexadecimal(length)  | custom script       | generates a random hexadecimal string of length `length`                              |
| $random.email                | `$randomEmail`      |                                                                                       |
| $exampleServer               | 🛑                  |                                                                                       |
