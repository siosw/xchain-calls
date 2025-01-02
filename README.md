# xchain-calls

- [x] deploy script (rust or sol?)
- [ ] generate valid intent in solidity
- [ ] generate valid intent in TS
- [ ] check against smart contracts
- [ ] filler service

## demo goals
- users can use funds on an origin chain to make calls (with their EOA as msg.sender) on a target chain

## high level design
- users create ERC7683 cross chain intent backed by assets on origin chain
- filler sets ERC7002 implementation on target chain (user provides auth data as part of intent)
- filler relays the intent (funded with origin chain assets) to destination chain via the set implementation

## implementation
- contracts taken from accross/xdelegate repo
- solver is a simple service scanning for on chain events + relaying the call (price check?)
- use a permissioned oracle for now

## next steps
- gasless orders w/ permit2
- better oracle / settlement
- practical demo application 
