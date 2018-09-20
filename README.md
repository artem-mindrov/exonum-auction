# Basic auction service

A very basic auction service implementation using the Exonum framework.
Currently supported operations:
 - create a wallet for an auction participant
 - create a lot belonging to a participant
 - place a bid
 - retrieve full bid history for a lot

## Description

The service API root is at ``/api/services/auction/v1`. Below is the current list of endpoints:
 | Endpoint                          | Operation                                            |
 |-----------------------------------|------------------------------------------------------|
 | `GET /wallet?pub_key={PublicKey}` | retrieve a wallet for the specified public key       |
 | `GET /wallets`                    | retrieve all wallets                                 |
 | `POST /wallets`                   | create a wallet using the specified public key       |
 | `POST /lots`                      | create a lot using the owner's public key            |
 | `POST /bids`                      | place a bid on a lot identified by the provided hash |
 | `GET /bids?id={Hash}`             | retrieve full bid history given a lot's tx hash      |

All POST requests in this table are asynchronous and only return the transaction hash associated with the request.
The only exception is `POST /bids` which will wait until the block is actually committed. Block height will be
returned in the `tx_block_height` property of the response.
Hashes returned by `POST /lots` are also used to identify the created lots and can be used to query their bid
information.

Mandatory entity body properties for the POST requests are as follows:

`POST /wallets`:

```
{
    "pub_key": <String>, // owner's public key
    "name": <String>,    // name
    "balance": <UInt64>  // starting balance
}
```

`POST /lots`:

```
{
    "pub_key": <String>, // owner's public key
    "name": <String>,    // name
    "min_bid": <UInt64>  // minimum starting bid amount
}
```

`POST /bids`:

```
{
    "owner": <String>, // public key of the participant placing the bid
    "lot": <String>,   // lot id (hash returned by POST /lots)
    "amount": <UInt64> // amount to bid, can only be greater than the current highest bid or the minimum starting bid
                       // this amount will be frozen until a higher bid is placed or the auction is closed
}
```