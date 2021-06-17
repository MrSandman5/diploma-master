#!/bin/bash

# Change this to the code ID of the auction contract for whatever chain your secretcli is using
contractcode="2"

# text colors
BLUE='\033[1;34m'
GRN='\033[1;32m'
NC='\033[0m'

# display auction info
display_auction_info() {
  auctioninfo=$(secretcli q compute query $auctionaddr '{"auction_info":{}}' --trust-node=true \
    -o json)
  echo -e "\n${BLUE}Auction Info:\n"
  echo "Sale Token:"
  echo -e "\tContract Address: ${GRN}$(jq -r '.auction_info.sell_token.contract_address' <<<$auctioninfo)"
  echo -e "\t${BLUE}Name: ${GRN}$(jq -r '.auction_info.sell_token.token_info.name' <<<$auctioninfo)"
  echo -e "\t${BLUE}Symbol: ${GRN}$(jq -r '.auction_info.sell_token.token_info.symbol' <<<$auctioninfo)"
  local saledecimals=$(jq -r '.auction_info.sell_token.token_info.decimals' <<<$auctioninfo)
  echo -e "\t${BLUE}Decimals: ${GRN}$saledecimals"
  echo -e "${BLUE}Bid Token:"
  echo -e "\tContract Address: ${GRN}$(jq -r '.auction_info.bid_token.contract_address' <<<$auctioninfo)"
  echo -e "\t${BLUE}Name: ${GRN}$(jq -r '.auction_info.bid_token.token_info.name' <<<$auctioninfo)"
  echo -e "\t${BLUE}Symbol: ${GRN}$(jq -r '.auction_info.bid_token.token_info.symbol' <<<$auctioninfo)"
  local buydecimals=$(jq -r '.auction_info.bid_token.token_info.decimals' <<<$auctioninfo)
  echo -e "\t${BLUE}Decimals: ${GRN}$buydecimals"
  local score=$(jq -r '.auction_info.score' <<<$auctioninfo)
  convert_denom $score $saledecimals
  echo -e "${BLUE}Score: ${GRN}$denom"
  local averagebid=$(jq -r '.auction_info.average_bid' <<<$auctioninfo)
  convert_denom $averagebid $buydecimals
  echo -e "${BLUE}Average Bid: ${GRN}$denom"
  local description=$(jq -r '.auction_info.description' <<<$auctioninfo)
  if [[ "$description" != "null" ]]; then
    echo -e "${BLUE}Description: ${GRN}$description"
  fi
  echo -e "${BLUE}Auction Address: ${GRN}$(jq -r '.auction_info.auction_address' <<<$auctioninfo)"
  echo -e "${BLUE}Status: ${GRN}$(jq -r '.auction_info.status' <<<$auctioninfo)${NC}"
  local winningbid=$(jq -r '.auction_info.winning_bid' <<<$auctioninfo)
  if [[ "$winningbid" != "null" ]]; then
    convert_denom $winningbid $buydecimals
    echo -e "${BLUE}Winning Bid: ${GRN}$denom${NC}\n"
  fi
}

# function to get a contract address, find its code hash and number of decimals
get_tokens() {
  read conaddr
  hash=$(secretcli q compute contract-hash "$conaddr" --trust-node=true -o json 2>&1)
  if echo "$hash" | grep ERROR; then
    goodinp=false
  else
    hash=${hash/0x/}
    local tokeninfo=$(secretcli q compute query "$conaddr" '{"token_info":{}}' \
      --trust-node=true -o json 2>&1)
    if echo $tokeninfo | grep ERROR; then
      echo -e "\nAre you sure that is a SNIP-20 token contract address?"
      goodinp=false
    else
      decimals=$(jq -r '.token_info.decimals' <<<"$tokeninfo")
      goodinp=true
    fi
  fi
}

get_contract() {
  read conaddr
  hash=$(secretcli q compute contract-hash "$conaddr" --trust-node=true -o json 2>&1)
  if echo "$hash" | grep ERROR; then
    goodinp=false
  else
    hash=${hash/0x/}
    goodinp=true
  fi
}

# use to check numerical input
re='^[0-9]*(\.[0-9]+)?$'

get_amount() {
  read inp
  if [[ "$inp" =~ $re ]]; then
    if [[ "$inp" == *.* ]]; then
      local dec=${inp#*.}
      local count=${#dec}
      if (($count > $1)); then
        echo -e "\nYOU ENTERED $count DECIMAL PLACES.\nTOKEN ONLY HAS $1 DECIMALS"
        return
      fi
    fi
    if ((${#inp} == 0)); then
      echo -e "\nINPUT MUST BE NUMERIC AND CAN NOT END WITH \".\""
      echo -e "EITHER DELETE THE \".\" OR MAKE IT \".0\""
      return
    fi
    amount=$(echo "$inp * 10^$1" | bc -l)
    amount=${amount%.*}
    goodinp=true
  else
    echo -e "\nINPUT MUST BE NUMERIC AND CAN NOT END WITH \".\""
    echo -e "EITHER DELETE THE \".\" OR MAKE IT \".0\""
  fi
}

# convert denom
convert_denom() {
  local convert=$(echo "$1 / 10^$2" | bc -l)
  denom=$(echo $convert | sed '/\./ s/\.\{0,1\}0\{1,\}$//')
}

cat <<EOF
Just a reminder that you need to have secretcli and jq installed.
You can install jq with:  sudo apt-get install jq

EOF

goodinp=false
while [ $goodinp == false ]; do
  echo -e "\nWhat is the secretcli keys alias of the account you want use?"
  read inp
  addr=$(secretcli q account $(secretcli keys show -a "$inp") --trust-node=true -o json |
    jq -r '.value.address')
  if echo $addr | grep secret; then
    addralias=$inp
    goodinp=true
  fi
done

goodinp=false
while [ $goodinp == false ]; do
  cat <<EOF


Would you like to:
create a new (a)uction
(l)ist existing auctions
create a new viewing (k)ey
show (b)alance
               (or (q)uit)
EOF
  read inp
  lowcase=$(echo $inp | awk '{print tolower($0)}')
  if [[ "$lowcase" == "auction" ]] || [[ "$lowcase" == "a" ]]; then
    cmd="a"
    goodinp=true
  elif [[ "$lowcase" == "list" ]] || [[ "$lowcase" == "l" ]]; then
    cmd="l"
    goodinp=true
  elif [[ "$lowcase" == "key" ]] || [[ "$lowcase" == "k" ]]; then
    cmd="k"
    goodinp=true
  elif [[ "$lowcase" == "balance" ]] || [[ "$lowcase" == "b" ]]; then
    cmd="b"
    goodinp=true
  elif [[ "$lowcase" == "quit" ]] || [[ "$lowcase" == "q" ]]; then
    exit
  fi
done

# list existing auctions
if [[ $cmd == 'l' ]]; then

  echo -e "\n"
  declare -A owners
  declare -A addrs
  auctionlist=$(secretcli q compute list-contract-by-code $contractcode --trust-node=true \
    -o json)
  if [[ "$auctionlist" == "null" ]]; then
    echo -e "\nThere are no auctions.  Try creating one!"
    exit
  fi
  contracttsv=$(jq -r '.[]|[.creator, .label, .address] | @tsv' <<<"$auctionlist")
  while IFS=$'\t' read -r creator label address; do
    echo $label
    owners+=([$label]=$creator)
    addrs+=([$label]=$address)
  done <<<"$contracttsv"

  # select auction
  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhich auction do you want to view?"
    read inp
    if [ ${owners[${inp}]+_} ]; then
      auctionlabel=$inp
      goodinp=true
    else
      jq -r '.[].label' <<<"$auctionlist"
      echo -e "\nAuction name \"$inp\" not found"
    fi
  done

  auctionowner=${owners[$auctionlabel]}
  auctionaddr=${addrs[$auctionlabel]}
  display_auction_info
  sellcontr=$(jq -r '.[].sell_token.contract_address' <<<"$auctioninfo")
  selldecimals=$(jq -r '.[].sell_token.token_info.decimals' <<<"$auctioninfo")
  bidcontr=$(jq -r '.[].bid_token.contract_address' <<<"$auctioninfo")
  biddecimals=$(jq -r '.[].bid_token.token_info.decimals' <<<"$auctioninfo")
  score=$(jq -r '.[].score' <<<"$auctioninfo")
  averagebid=$(jq -r '.[].average_bid' <<<"$auctioninfo")
  auctionstat=$(jq -r '.[].status' <<<"$auctioninfo")

  # display options for the owner
  if [[ "$auctionowner" == "$addr" ]]; then
    while [ true ]; do
      goodinp=false
      while [ $goodinp == false ]; do
        cat <<EOF

Would you like to:
(c)onsign tokens to auction escrow
(f)inalize/close the auction
(d)isplay auction info
               (or (q)uit)
EOF
        read inp
        lowcase=$(echo $inp | awk '{print tolower($0)}')
        if [[ "$lowcase" == "consign" ]] || [[ "$lowcase" == "c" ]]; then
          owncmd="c"
          goodinp=true
        elif [[ "$lowcase" == "finalize" ]] || [[ "$lowcase" == "f" ]]; then
          owncmd="f"
          goodinp=true
        elif [[ "$lowcase" == "display" ]] || [[ "$lowcase" == "d" ]]; then
          owncmd="d"
          goodinp=true
        elif [[ "$lowcase" == "quit" ]] || [[ "$lowcase" == "q" ]]; then
          exit
        fi
      done
      # finalize/close auction
      if [[ $owncmd == 'f' ]]; then
        goodinp=false
        while [ $goodinp == false ]; do
          echo -e "\nDo you want to keep the auction open if there are currently no active bids?"
          echo "(y)es or (n)o"
          read keepopen
          lowcase=$(echo $keepopen | awk '{print tolower($0)}')
          if [[ "$lowcase" == "yes" ]] || [[ "$lowcase" == "y" ]]; then
            onlyif=true
            goodinp=true
          elif [[ "$lowcase" == "no" ]] || [[ "$lowcase" == "n" ]]; then
            onlyif=false
            goodinp=true
          fi
        done
        #
        # change --gas amount below if getting out of gas error during finalize/close
        #
        resp=$(secretcli tx compute execute $auctionaddr \
          "{\"finalize\":{\"only_if_bids\":$onlyif}}" --from $addr --gas 2000000 \
          --broadcast-mode block --trust-node=true -o json -y)
        echo "$resp" | grep "out of gas"
        tx=$(jq -r '.txhash' <<<"$resp")
        decd=$(secretcli q compute tx $tx --trust-node=true -o json)
        fnlresp=$(jq -r '.output_data_as_string' <<<"$decd")
        fnlresp=${fnlresp//\\"/"/}
        echo -e "${BLUE}Finalize:\n"
        echo -e "Status: ${GRN}$(jq -r '.close_auction.status' <<<$fnlresp)"
        echo -e "${BLUE}Message: ${GRN}$(jq -r '.close_auction.message' <<<$fnlresp)${NC}"
        fnlwinningbid=$(jq -r '.close_auction.winning_bid' <<<$fnlresp)
        if [[ "$fnlwinningbid" != "null" ]]; then
          convert_denom $fnlwinningbid $biddecimals
          echo -e "${BLUE}Winning Bid: ${GRN}$denom${NC}"
        fi
        fnlreturned=$(jq -r '.close_auction.amount_returned' <<<$fnlresp)
        if [[ "$fnlreturned" != "null" ]]; then
          convert_denom $fnlreturned $selldecimals
          echo -e "${BLUE}Amount Returned: ${GRN}$denom${NC}"
        fi
        # consign tokens
      elif [[ $owncmd == 'c' ]]; then
        goodinp=false
        while [ $goodinp == false ]; do
          echo -e "\nHow much do you want to consign?"
          echo "Recommend consigning the full sale amount, but you can do it in multiple"
          echo -e "transactions if you want\n"
          echo "Enter amount of tokens according to contract decimals (count of uSCRT)"
          echo -e "Example (standart): 1 SNIP20 token = 1000000 uSCRT\n"
          get_amount $selldecimals
        done
        csnamount=$amount
        #
        # change --gas amount below if getting out of gas error during consign
        #
        resp=$(secretcli tx compute execute $sellcontr "{\"send\":{\"recipient\":\
\"$auctionaddr\",\"amount\":\"$csnamount\"}}" --from $addr --gas 500000 \
          --broadcast-mode block --trust-node=true -o json -y)
        echo "$resp" | grep "out of gas"
        sendtx=$(jq -r '.txhash' <<<"$resp")
        decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
        decdsenderr=$(jq '.output_error' <<<"$decdsend")
        if [[ "$decdsenderr" == "{}" ]]; then
          padkey=$(printf "%-256s" "response")
          logresp=$(jq -r --arg KEY "$padkey" \
            '.output_log[0].attributes[]|select(.key==$KEY).value' <<<"$decdsend")
          cleaned=$(echo $logresp | sed 's/\\//g')
          echo -e "${BLUE}Consign:\n"
          echo -e "Status: ${GRN}$(jq -r '.consign.status' <<<$cleaned)"
          echo -e "${BLUE}Message: ${GRN}$(jq -r '.consign.message' <<<$cleaned)${NC}"
          csnamtcsn=$(jq -r '.consign.amount_consigned' <<<$cleaned)
          if [[ "$csnamtcsn" != "null" ]]; then
            convert_denom $csnamtcsn $selldecimals
            echo -e "${BLUE}Amount Consigned: ${GRN}$denom${NC}"
          fi
          csnamtneed=$(jq -r '.consign.amount_needed' <<<$cleaned)
          if [[ "$csnamtneed" != "null" ]]; then
            convert_denom $csnamtneed $selldecimals
            echo -e "${BLUE}Amount Needed: ${GRN}$denom${NC}"
          fi
          csnreturned=$(jq -r '.consign.amount_returned' <<<$cleaned)
          if [[ "$csnreturned" != "null" ]]; then
            convert_denom $csnreturned $selldecimals
            echo -e "${BLUE}Amount Returned: ${GRN}$denom${NC}"
          fi
        else
          echo $decdsenderr
        fi
        # display auction info
      elif [[ $owncmd == 'd' ]]; then
        display_auction_info
      fi
    done
    # display options for bidder
  else
    while [ true ]; do
      goodinp=false
      while [ $goodinp == false ]; do
        cat <<EOF

Would you like to:
(p)lace a new bid
(v)iew an active bid
(d)isplay auction info
               (or (q)uit)
EOF
        read inp
        lowcase=$(echo $inp | awk '{print tolower($0)}')
        if [[ "$lowcase" == "place" ]] || [[ "$lowcase" == "p" ]]; then
          bidcmd="p"
          goodinp=true
        elif [[ "$lowcase" == "view" ]] || [[ "$lowcase" == "v" ]]; then
          bidcmd="v"
          goodinp=true
        elif [[ "$lowcase" == "display" ]] || [[ "$lowcase" == "d" ]]; then
          bidcmd="d"
          goodinp=true
        elif [[ "$lowcase" == "quit" ]] || [[ "$lowcase" == "q" ]]; then
          exit
        fi
      done
      # place a bid
      if [[ $bidcmd == 'p' ]]; then
        goodinp=false
        while [ $goodinp == false ]
        do
            echo -e "\nHow much do you want to bid?"
            echo "Your bid should be expected payments-to-proposal ratio times lesser power of 10."
            echo "You can't expect getting lesser or equal to what you propose."
            echo "Enter amount of tokens according to contract decimals (count of uSCRT)"
            echo -e "Example (standart): 1 SNIP20 token = 1000000 uSCRT\n"
            get_amount $biddecimals
        done
        bidamount=$amount

        # need to add padding to hide bid length, Uint128 can have about 40 digits
        bidlen=${#bidamount}
        missing=$(( 40 - bidlen ))
        spaces=$(printf '%*s' $missing)
#
# change --gas amount below if getting out of gas error during place bid
#
        resp=$(secretcli tx compute execute $bidcontr "{\"send\":{\"recipient\":\
                  \"$auctionaddr\",\"amount\":\"$bidamount\",\"padding\":\"$spaces\"}}"\
                   --from $addr --gas 500000 --broadcast-mode block --trust-node=true -o json -y)

        echo "$resp" | grep "out of gas"
        sendtx=$(jq -r '.txhash' <<<"$resp")
        decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
        decdsenderr=$(jq '.output_error' <<<"$decdsend")
        if [[ "$decdsenderr" == "{}" ]]; then
          padkey=$(printf "%-256s" "response")
          logresp=$(jq -r --arg KEY "$padkey" \
            '.output_log[0].attributes[]|select(.key==$KEY).value' <<<"$decdsend")
          cleaned=$(echo $logresp | sed 's/\\//g')
          echo -e "${BLUE}Bid:\n"
          echo -e "Status: ${GRN}$(jq -r '.bid.status' <<<$cleaned)"
          echo -e "${BLUE}Message: ${GRN}$(jq -r '.bid.message' <<<$cleaned)${NC}"
          prevbid=$(jq -r '.bid.previous_bid' <<<$cleaned)
          if [[ "$prevbid" != "null" ]]; then
            convert_denom $prevbid $biddecimals
            echo -e "${BLUE}Previous Bid: ${GRN}$denom${NC}"
          fi
          amountbid=$(jq -r '.bid.amount_bid' <<<$cleaned)
          if [[ "$amountbid" != "null" ]]; then
            convert_denom $amountbid $biddecimals
            echo -e "${BLUE}Amount Bid: ${GRN}$denom${NC}"
          fi
          bidreturned=$(jq -r '.bid.amount_returned' <<<$cleaned)
          if [[ "$bidreturned" != "null" ]]; then
            convert_denom $bidreturned $biddecimals
            echo -e "${BLUE}Amount Returned: ${GRN}$denom${NC}"
          fi
        else
          echo $decdsenderr
        fi
        # display auction info
      elif [[ $bidcmd == 'd' ]]; then
        display_auction_info

        # view active bid
      elif [[ $bidcmd == 'v' ]]; then
        #
        # change --gas amount below if getting out of gas error during view bid
        #
        resp=$(secretcli tx compute execute $auctionaddr '{"view_bid":{}}' --from $addr --gas \
          200000 --broadcast-mode block --trust-node=true -o json -y)
        echo "$resp" | grep "out of gas"
        tx=$(jq -r '.txhash' <<<"$resp")
        decd=$(secretcli q compute tx $tx --trust-node=true -o json)
        bidresp=$(jq -r '.output_data_as_string' <<<"$decd")
        bidresp=${bidresp//\\"/"/}
        echo -e "${BLUE}Bid:\n"
        echo -e "Status: ${GRN}$(jq -r '.bid.status' <<<$bidresp)"
        echo -e "${BLUE}Message: ${GRN}$(jq -r '.bid.message' <<<$bidresp)${NC}"
        amountbid=$(jq -r '.bid.amount_bid' <<<$bidresp)
        if [[ "$amountbid" != "null" ]]; then
          convert_denom $amountbid $biddecimals
          echo -e "${BLUE}Amount Bid: ${GRN}$denom${NC}"
        fi
      fi
    done
  fi
# create new auction
elif [[ $cmd == 'a' ]]; then
  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat is the contract address of the token you want to sell?"
    get_tokens
  done
  selladdr=$conaddr
  sellhash=$hash
  selldecimals=$decimals

  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat is the contract address of the token you will accept bids in?"
    get_tokens
  done
  bidaddr=$conaddr
  bidhash=$hash
  biddecimals=$decimals

  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nHow much you want to get?"
    read expected
    goodinp=true
  done

  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nHow much you expect to pay in total?"
    read payment
    goodinp=true
  done

  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat is the contract address of the oracle?"
    get_contract
  done
  oracleaddr=$conaddr
  oraclehash=$hash

  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nDo you want to add an optional free-form text description?"
    echo "(y)es or (n)o"
    read wantdesc
    lowcase=$(echo $wantdesc | awk '{print tolower($0)}')
    if [[ "$lowcase" == "yes" ]] || [[ "$lowcase" == "y" ]]; then
      echo -e "\nPlease enter your description without quotes"
      read desc
      descinp=$desc
      goodinp=true
    elif [[ "$lowcase" == "no" ]] || [[ "$lowcase" == "n" ]]; then
      descinp=""
      goodinp=true
    fi
  done
  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat label would you like to give your auction?"
    read auctionlabel
    #
    # change --gas amount below if getting out of gas error when creating a new auction
    #
    resp=$(secretcli tx compute instantiate $contractcode "{\"sell_contract\":{\"code_hash\":\
            \"$sellhash\",\"address\":\"$selladdr\"},\"bid_contract\":{\"code_hash\":\"$bidhash\",\"address\":\
            \"$bidaddr\"},\"expected\":\"$expected\",\"payment\":\"$payment\",\"oracle_contract\":{\"code_hash\":\
            \"$oraclehash\",\"address\":\"$oracleaddr\"},\"description\":\"$descinp\"}" --from $addr \
      --label "$auctionlabel" --gas 300000 --broadcast-mode block --trust-node=true \
      -o json -y 2>&1)
    if echo $resp | grep "label already exists"; then
      true
    else
      if echo $resp | grep "out of gas"; then
        exit
      elif echo $resp | grep ERROR; then
        exit
      elif echo $resp | grep "failed to execute message"; then
        sendtx=$(jq -r '.txhash' <<<"$resp")
        decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
        jq '.output_error' <<<"$decdsend"
        exit
      else
        goodinp=true
        auctionlist=$(secretcli q compute list-contract-by-code $contractcode \
          --trust-node=true -o json)
        auctionaddr=$(jq -r --arg AUC "$auctionlabel" '.[] | select(.label==$AUC).address' \
          <<<"$auctionlist")
        display_auction_info
      fi
    fi
  done

# create viewing key for token
elif [[ $cmd == 'k' ]]; then
  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat is the contract address of token contact you use?"
    get_tokens
  done
  tokenaddr=$conaddr

  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat is the random phrase for your key you want to use?"
    read entropy

    resp=$(secretcli tx compute execute $tokenaddr "{\"create_viewing_key\": {\"entropy\": \"$entropy\"}}" --from $addr \
      --broadcast-mode block --trust-node=true -o json -y 2>&1)
    if echo $resp | grep "label already exists"; then
      true
    else
      if echo $resp | grep ERROR; then
        exit
      elif echo $resp | grep "failed to execute message"; then
        sendtx=$(jq -r '.txhash' <<<"$resp")
        decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
        jq '.output_error' <<<"$decdsend"
        exit
      else
        sendtx=$(jq -r '.txhash' <<<"$resp")
        decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
        bidresp=$(jq -r '.output_data_as_string' <<<"$decdsend")
        bidresp=${bidresp//\\"/"/}
        echo -e "\nViewing key for node \"$addralias\": ${GRN}$(jq -r '.create_viewing_key.key' <<<$bidresp)${NC}\n"
        goodinp=true
      fi
    fi
  done

# check balance
elif [[ $cmd == 'b' ]]; then
  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat is the contract address of token contact you use?"
    get_tokens
  done
  tokenaddr=$conaddr

  goodinp=false
  while [ $goodinp == false ]; do
    echo -e "\nWhat is the your viewing key?"
    read viewing_key

    resp=$(secretcli q compute query $tokenaddr "{\"balance\": {\"address\": \"$addr\", \"key\": \"$viewing_key\"}}")
    echo -e "\nBalance from \"$tokenaddr\" token for node \"$addralias\": ${GRN}$(jq -r '.balance.amount' <<<$resp)${NC}\n"
    goodinp=true
  done
fi

exit
