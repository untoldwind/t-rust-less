@0x89ae7248ac2e8067;

enum CipherType {
    rsaAesGcm @0;
    ed25519Chacha20Poly1305 @1;
}

struct PublicKey {
    type @0 : CipherType;
    key @1 : Data;
}

struct Recipient {
    id @0 : Data;
    name @1 : Text;
    email @2 : Text;
    publicKeys @3 : List(PublicKey);
}

struct PublicRing {
    recipients @0 : List(Recipient);

}

struct Ring {
    recipients @0 : List(User);

    struct User {
        id @0 : Data;
        privateKeys @1 : List(PrivateKey);
    }

    struct PrivateKey {
        type @0 : CipherType;
        salt @1 : Data;
        cryptedKey @2 : Data;
    }

}

struct Block {
    header @0 : List(Header);
    cryptedContent @1 : Data;

    struct Header {
        type @0 : Header;
        keyCommon @1 : Data;
        recipientKeys @2 : List(RecipientKey);        
    }

    struct RecipientKey {
        id @0 : Data;
        key @1 : Data;
    }
}