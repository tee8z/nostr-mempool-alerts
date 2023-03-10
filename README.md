bot:
What is should be able to do:
* someone should be able to post to the bot, ask it to send the notifications to a given relay
* 3 types of notifications:
    * ask to be alerted a given block height has been reached
    * ask to be alerted when mempool fees have reached a given level
    * ask when a transaction has reached a certain number of confirmations
* the user of the bot may ask for one type of alert at a time
* a user may ask for as many alerts as they like, there will be duplication protection around the alerting (ie if they ask for the same alert it will be ignored)


This bot server should be very easy for someone to run (package as a binary), so anyone can run their own alerting bot connected to their own relay & own mempool.space instance.

Plan to have a public one people can use


* Future plan to add a utxo movement tracking option (this one is trickier as the data to determine is user specific)