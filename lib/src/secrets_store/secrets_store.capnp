@0x89ae7248ac2e8067;

# Enumeration of all cipher suites
enum KeyType {
    rsaAesGcm @0;
    ed25519Chacha20Poly1305 @1;
}

# Public key of a specific cipher suite
struct PublicKey {
    type @0 : KeyType;
    key @1 : Data;
}

# Recipient of encrypted data
struct Recipient {
    id @0 : Text;
    name @1 : Text;
    email @2 : Text;
    publicKeys @3 : List(PublicKey);
}

# Layout of a public ring block
struct PublicRing {
    recipients @0 : List(Recipient);
}

# Layout of a (private ring block)
struct Ring {
    users @0 : List(User);

    struct User {
        recipient @0 : Recipient;
        privateKeys @1 : List(PrivateKey);
    }

    struct PrivateKey {
        type @0 : KeyType;
        preset @1 : UInt8;
        nonce @2 : Data;
        cryptedKey @3 : Data;
    }
}

# Layout of a data block
# Since "Data" is reserved by Cap'n Proto, we just call it Block
struct Block {
    headers @0 : List(Header);
    content @1 : Data;

    struct Header {
        type @0 : KeyType;
        commonKey @1 : Data;
        recipients @2 : List(RecipientKey);
    }

    struct RecipientKey {
        id @0 : Text;
        cryptedKey @1: Data;
    }
}