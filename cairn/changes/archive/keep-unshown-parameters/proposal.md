---
cairn: change
id: keep-unshown-parameters
status: landed
created: 2026-08-29
---

# A modelled property loses every parameter the form does not show

## Why

The projection's own header promises that "properties and component types tcal does not model are kept verbatim". That holds for a property outside the vocabulary, and a law pins it. It does not hold for the parameters of a property inside it: the form has keys for a handful, and folding the document back rebuilt the line out of those keys alone, so every other parameter went.

    DESCRIPTION;ALTREP="cid:part1":d            ->  DESCRIPTION:d
    ORGANIZER;CN=Chair;SENT-BY="mailto:s@x":..  ->  ORGANIZER:mailto:chair@example.com
    ATTENDEE;RSVP=TRUE;CUTYPE=INDIVIDUAL;..     ->  ATTENDEE;PARTSTAT=ACCEPTED:..
    SUMMARY;LANGUAGE=en:a summary               ->  SUMMARY:a summary

`RSVP` is what tells a scheduling client whether to ask the attendee at all, and `SENT-BY` is who is allowed to speak for the organiser, the same authority the merge takes seriously enough to refuse a change over (RFC 5546 section 3.2). Deciding a conflict by hand dropped both without a word, which undoes by editing what the merge did deliberately.

## What

Patch the line the value came from rather than rebuilding it. A parameter the form has a key for is the document's to set, taken from the rebuilt line and dropped where the document cleared it. Every other parameter is the line's own and stays where it stood, so an untouched property is byte-identical and an edited one keeps everything the form never asked about.

tCard has the same gap and is being fixed the same way, in a module of the same name, since the two crates are one design over two formats.
