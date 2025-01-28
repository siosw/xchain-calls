# xchain-calls

## demo goals
users can use a dapp on the destination chain while only sending txs on the origin chain

## cli

## high level design
- users create ERC7683 cross chain message backed by assets on origin chain
- filler sets ERC7002 implementation on target chain (user provides auth data as part of the order)
- filler relays the order to destination chain via the set implementation and gets rewarded with the origin assets

## contracts 
contracts taken from accross/xdelegate repo

## next steps
- gasless orders w/ permit2
- better oracle / settlement
- practical demo application
