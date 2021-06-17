#!/bin/bash

contractcode="3"

cat << EOF
Just a reminder that you need to have secretcli and jq installed.
You can install jq with:  sudo apt-get install jq

EOF

goodinp=false
while [ $goodinp == false ]
do
    echo -e "\nWhat is the secretcli keys alias of the account you want use?"
    read inp
    addr=$(secretcli q account $(secretcli keys show -a "$inp") --trust-node=true -o json \
           | jq -r '.value.address')
    if echo $addr | grep secret
    then
        goodinp=true
    fi
done

goodinp=false
while [ $goodinp == false ]
do
   cat << EOF


Would you like to:
initialize (o)racle
add a new (h)istory
(s)how user history
               (or (q)uit)
EOF
    read inp
    lowcase=$(echo "$inp" | awk '{print tolower($0)}')
    if [[ "$lowcase" == "oracle" ]] || [[ "$lowcase" == "o" ]]
    then
        cmd="o"
        goodinp=true
    elif [[ "$lowcase" == "history" ]] || [[ "$lowcase" == "h" ]]
    then
        cmd="h"
        goodinp=true
    elif [[ "$lowcase" == "show" ]] || [[ "$lowcase" == "s" ]]
    then
        cmd="s"
        goodinp=true
    elif [[ "$lowcase" == "quit" ]] || [[ "$lowcase" == "q" ]]
    then
        exit
    fi
done

if [[ $cmd == 'o' ]]
then
    goodinp=false
    while [ $goodinp == false ]
    do
        resp=$(secretcli tx compute instantiate $contractcode "{}" --from "$addr" \
            --label "oracle" --gas 300000 --broadcast-mode block --trust-node=true \
            -o json -y 2>&1)
        if echo $resp | grep "label already exists"
        then
            true
        else
            if echo $resp | grep "out of gas"
            then
                exit
            elif echo $resp | grep ERROR
            then
                exit
            elif echo $resp | grep "failed to execute message"
            then
                sendtx=$(jq -r '.txhash' <<<$resp)
                decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
                jq '.output_error' <<<"$decdsend"
                exit
            else
                echo -e "\n$resp\n"
            fi
        fi
    goodinp=true
    done

elif [[ $cmd == 'h' ]]
then
    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the oracle address?"
        read oracle_address
        goodinp=true
    done

    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the user address?"
        read user
        goodinp=true
    done

    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the user's credit history?"
        read history

        resp=$(secretcli tx compute execute "$oracle_address" "{\"add_history\": {\"user\": \"$user\", \"history\": $history}}" --from $addr \
            --broadcast-mode block --trust-node=true -o json -y 2>&1)
        echo -e "\n$resp\n"
        goodinp=true
    done

elif [[ $cmd == 's' ]]
then

    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the oracle address?"
        read oracle_address
        goodinp=true
    done

    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the user address?"
        read user
        goodinp=true
    done

    resp=$(secretcli q compute query "$oracle_address" "{\"get_history\": {\"user\": \"$user\"}}" \
             --trust-node=true -o json)
    echo -e "\n$resp\n"
fi

exit
