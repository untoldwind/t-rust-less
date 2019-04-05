@0x89ae7248ac2e8067;

struct PublicKey {
    type @0 : KeyType;
    key @1 : KeyType;
}

enum KeyType {
    rsaAesGcm @0;
    ed25519Chacha20Poly1305 @1;
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
        type @0 : KeyType;
        salt @1 : Data;
        cryptedKey @2 : Data;
    }

}

struct Block {
    keys @0 : List(BlockKey);
    cryptedContent @1 : Data;

    struct BlockKey {
        type @0 : BlockKeyType;
        salt @1 : Data;
        cryptedKey @2: Data;
    }

    struct Recipient {
        id @0 : Data;
        publicBlockKeys @1 : List(PublicKey);
    }

    enum BlockKeyType {
        aesGcm @0;
        chacha20Poly1305 @1;
    }
}